#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]
#![cfg_attr(feature = "interface", feature(negative_impls))]

mod entity;
mod event;
mod hotkey;
mod items;
mod message;
mod packet_versions;
mod server;

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use event::{
    CharacterServerDisconnectedEvent, DisconnectedEvent, LoginServerDisconnectedEvent, MapServerDisconnectedEvent, NetworkEventList,
};
use ragnarok_bytes::encoding::UTF_8;
use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes};
use ragnarok_packets::handler::{DuplicateHandlerError, HandlerResult, NoPacketCallback, PacketCallback, PacketHandler};
use ragnarok_packets::*;
use server::{ServerConnectCommand, ServerConnection};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

pub use self::entity::EntityData;
pub use self::event::{DisconnectReason, NetworkEvent};
pub use self::hotkey::HotkeyState;
pub use self::items::{InventoryItem, InventoryItemDetails, ItemQuantity, NoMetadata, SellItem, ShopItem};
pub use self::message::MessageColor;
pub use self::packet_versions::SupportedPacketVersion;
pub use self::server::{
    CharacterServerLoginData, LoginServerLoginData, NotConnectedError, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};
use crate::server::NetworkTaskError;

/// Buffer for networking events. This struct exists to reduce heap allocations
/// and is purely an optimization.
pub struct NetworkEventBuffer(Vec<NetworkEvent>);

impl NetworkEventBuffer {
    pub fn drain(&mut self) -> std::vec::Drain<'_, NetworkEvent> {
        self.0.drain(..)
    }
}

/// Simple time synchronization using the Cristian's algorithm.
struct TimeSynchronization {
    request_send: Instant,
    request_received: Instant,
    client_tick: f64,
}

impl TimeSynchronization {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            request_send: now,
            request_received: now,
            client_tick: 100.0,
        }
    }

    /// Returns the client tick that must be used when sending the time
    /// synchronization request immediately after calling this function.
    fn request_client_tick(&mut self) -> u32 {
        let request_send = Instant::now();
        let elapsed = request_send.duration_since(self.request_received).as_secs_f64();
        (self.client_tick + (elapsed * 1000.0)) as u32
    }

    /// Returns the estimated client tick using the Cristian's algorithm.
    fn estimated_client_tick(&mut self, server_tick: u32, request_received: Instant) -> u32 {
        self.request_received = request_received;
        let round_trip_time = self.request_received.duration_since(self.request_send).as_secs_f64();
        let tick_adjustment = (round_trip_time / 2.0) * 1000.0;
        self.client_tick = f64::from(server_tick) + tick_adjustment;
        self.client_tick as u32
    }
}

pub struct NetworkingSystem<Callback> {
    command_sender: UnboundedSender<ServerConnectCommand>,
    time_synchronization: Arc<Mutex<TimeSynchronization>>,
    login_server_connection: ServerConnection,
    character_server_connection: ServerConnection,
    map_server_connection: ServerConnection,
    /// Which restart packet was sent last, together with the send time. Used to
    /// tell a `0x00B3` restart ack for a disconnect apart from one
    /// answering a respawn request.
    last_restart: Option<(RestartType, Instant)>,
    packet_callback: Callback,
}

impl NetworkingSystem<NoPacketCallback> {
    pub fn spawn() -> (Self, NetworkEventBuffer) {
        let (command_sender, time_synchronization) = Self::spawn_networking_thread(NoPacketCallback);
        Self::inner_new(command_sender, time_synchronization, NoPacketCallback)
    }
}

impl<Callback> NetworkingSystem<Callback>
where
    Callback: PacketCallback + Send,
{
    fn inner_new(
        command_sender: UnboundedSender<ServerConnectCommand>,
        time_synchronization: Arc<Mutex<TimeSynchronization>>,
        packet_callback: Callback,
    ) -> (Self, NetworkEventBuffer) {
        let networking_system = Self {
            command_sender,
            time_synchronization,
            login_server_connection: ServerConnection::Disconnected,
            character_server_connection: ServerConnection::Disconnected,
            map_server_connection: ServerConnection::Disconnected,
            last_restart: None,
            packet_callback,
        };
        let event_buffer = NetworkEventBuffer(Vec::new());

        (networking_system, event_buffer)
    }

    pub fn spawn_with_callback(packet_callback: Callback) -> (Self, NetworkEventBuffer) {
        let (command_sender, time_synchronization) = Self::spawn_networking_thread(packet_callback.clone());
        Self::inner_new(command_sender, time_synchronization, packet_callback)
    }

    fn spawn_networking_thread(packet_callback: Callback) -> (UnboundedSender<ServerConnectCommand>, Arc<Mutex<TimeSynchronization>>) {
        let (command_sender, mut command_receiver) = tokio::sync::mpsc::unbounded_channel::<ServerConnectCommand>();
        let time_synchronization = Arc::new(Mutex::new(TimeSynchronization::new()));
        let thread_time_synchronization = Arc::clone(&time_synchronization);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

            let _guard = runtime.enter();
            let local_set = tokio::task::LocalSet::new();

            let mut login_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;
            let mut character_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;
            let mut map_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;

            local_set.block_on(&runtime, async {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        ServerConnectCommand::Login {
                            address,
                            action_receiver,
                            event_sender,
                            packet_version,
                        } => {
                            if let Some(handle) = login_server_task_handle.take() {
                                // TODO: Maybe add a timeout here?
                                // Ignore join errors (including panics in the previous task) so
                                // the networking thread keeps running and new connections work.
                                let _ = handle.await;
                            }

                            let packet_handler = Self::create_login_server_packet_handler(packet_callback.clone(), packet_version).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                |_| LoginServerKeepalivePacket::new(),
                                Duration::from_secs(58),
                                false,
                                thread_time_synchronization.clone(),
                            ));

                            login_server_task_handle = Some(handle);
                        }
                        ServerConnectCommand::Character {
                            address,
                            action_receiver,
                            event_sender,
                            packet_version,
                        } => {
                            if let Some(handle) = character_server_task_handle.take() {
                                // TODO: Maybe add a timeout here?
                                // Ignore join errors (including panics in the previous task) so
                                // the networking thread keeps running and new connections work.
                                let _ = handle.await;
                            }

                            let packet_handler =
                                Self::create_character_server_packet_handler(packet_callback.clone(), packet_version).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                |_| CharacterServerKeepalivePacket::new(),
                                Duration::from_secs(10),
                                true,
                                thread_time_synchronization.clone(),
                            ));

                            character_server_task_handle = Some(handle);
                        }
                        ServerConnectCommand::Map {
                            address,
                            action_receiver,
                            event_sender,
                            packet_version,
                        } => {
                            if let Some(handle) = map_server_task_handle.take() {
                                // TODO: Maybe add a timeout here?
                                // Ignore join errors (including panics in the previous task) so
                                // the networking thread keeps running and new connections work.
                                let _ = handle.await;
                            }

                            let packet_handler = Self::create_map_server_packet_handler(packet_callback.clone(), packet_version).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                |time_synchronization| match time_synchronization.lock() {
                                    Ok(mut time_synchronization) => {
                                        let client_tick = time_synchronization.request_client_tick();
                                        RequestServerTickPacket::new(ClientTick(client_tick))
                                    }
                                    Err(_) => RequestServerTickPacket::new(ClientTick(100)),
                                },
                                Duration::from_secs(10),
                                false,
                                thread_time_synchronization.clone(),
                            ));

                            map_server_task_handle = Some(handle);
                        }
                    }
                }
            });
        });

        (command_sender, time_synchronization)
    }

    fn handle_connection<Event>(connection: &mut ServerConnection, event_buffer: &mut NetworkEventBuffer)
    where
        Event: DisconnectedEvent,
    {
        match connection.take() {
            ServerConnection::Connected {
                action_sender,
                mut event_receiver,
                packet_version,
            } => loop {
                match event_receiver.try_recv() {
                    Ok(login_event) => {
                        event_buffer.0.push(login_event);
                    }
                    Err(TryRecvError::Empty) => {
                        *connection = ServerConnection::Connected {
                            action_sender,
                            event_receiver,
                            packet_version,
                        };
                        break;
                    }
                    Err(..) => {
                        event_buffer.0.push(Event::create_event(DisconnectReason::ConnectionError));
                        *connection = ServerConnection::Disconnected;
                        break;
                    }
                }
            },
            ServerConnection::ClosingManually => {
                event_buffer.0.push(Event::create_event(DisconnectReason::ClosedByClient));
                *connection = ServerConnection::Disconnected;
            }
            _ => (),
        };
    }

    pub fn get_events(&mut self, events: &mut NetworkEventBuffer) {
        let first_new_event = events.0.len();

        Self::handle_connection::<LoginServerDisconnectedEvent>(&mut self.login_server_connection, events);
        Self::handle_connection::<CharacterServerDisconnectedEvent>(&mut self.character_server_connection, events);
        Self::handle_connection::<MapServerDisconnectedEvent>(&mut self.map_server_connection, events);

        let Some((restart_type, requested_at)) = self.last_restart.take() else {
            return;
        };

        // If the answer never arrives (e.g. the map connection died), don't keep the
        // state around forever.
        if requested_at.elapsed() > Duration::from_secs(15) {
            return;
        }

        let has_logged_out = events.0[first_new_event..]
            .iter()
            .any(|event| matches!(event, NetworkEvent::LoggedOut));

        if !has_logged_out {
            // Put the state back, we are still waiting for the answer.
            self.last_restart = Some((restart_type, requested_at));
            return;
        }

        if matches!(restart_type, RestartType::Respawn) {
            // `0x00B3` (ZC_RESTART_ACK) is also sent for a respawn request. Treating it as
            // "logged out" disconnected the client from the map server right after clicking
            // Respawn, which left the game on a black screen (the map change/spawn that
            // follows the respawn was never processed). Drop the bogus logout event; the
            // server sends the normal map change or resurrection right after the ack.
            let tail: Vec<NetworkEvent> = events.0.split_off(first_new_event);
            events
                .0
                .extend(tail.into_iter().filter(|event| !matches!(event, NetworkEvent::LoggedOut)));
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_server_connection<PingPacket>(
        address: SocketAddr,
        mut action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<NetworkEvent>,
        mut packet_handler: PacketHandler<NetworkEventList, Callback>,
        ping_factory: impl Fn(&Mutex<TimeSynchronization>) -> PingPacket,
        ping_frequency: Duration,
        // After logging in to the character server, it sends the account id without any packet.
        // Since our packet handler has no way of working with this, we need to add some special
        // logic.
        mut read_account_id: bool,
        time_synchronization: Arc<Mutex<TimeSynchronization>>,
    ) -> Result<(), NetworkTaskError>
    where
        PingPacket: Packet + ClientPacket,
        Callback: PacketCallback,
    {
        let mut stream = TcpStream::connect(address).await.map_err(|_| NetworkTaskError::FailedToConnect)?;
        let mut interval = tokio::time::interval(ping_frequency);
        // A tokio interval fires immediately on the first tick, which would race the
        // ping against the login packet. rAthena's map server requires the first read
        // of a session to contain exactly the login packet, so delay the first ping
        // by one full period.
        interval.reset();
        let mut buffer = [0u8; 8192];
        let mut cut_off_buffer_base = 0;
        let mut events = Vec::new();
        let mut byte_writer = ByteWriter::with_encoding(UTF_8);

        loop {
            tokio::select! {
                // Send a packet to the server.
                action = action_receiver.recv() => {
                    let Some(action) = action else {
                        // Channel was closed by the main thread.
                        break Ok(());
                    };

                    stream.write_all(&action).await.map_err(|_| NetworkTaskError::ConnectionClosed)?;
                }
                // Receive some packets from the server.
                received_bytes = stream.read(&mut buffer[cut_off_buffer_base..]) => {
                    let Ok(received_bytes) = received_bytes else {
                        // Channel was closed by the main thread.
                        break Err(NetworkTaskError::ConnectionClosed);
                    };

                    if received_bytes == 0 {
                        // Receiving Ok(0) means the stream was closed by the server, most
                        // likely because the client sent an incorrect packet.
                        break Err(NetworkTaskError::ConnectionClosed);
                    }

                    let data_len = cut_off_buffer_base + received_bytes;
                    // Offset of the tail that must be kept to complete a packet that
                    // was split across two TCP reads (`None` when everything in the
                    // buffer was consumed).
                    let mut keep_tail_from: Option<usize> = None;
                    // Set when the stream is in an unknown state (unsupported or
                    // malformed packet): the remainder of the buffer cannot be
                    // interpreted and is discarded.
                    let mut discard_remainder = false;

                    {
                        let mut byte_reader = ByteReader::without_metadata(&buffer[..data_len]);
                        byte_reader.set_encoding(UTF_8);

                        if read_account_id {
                            if data_len < 4 {
                                // The 4-byte account id echo has not fully arrived yet. Keep
                                // the bytes for the next read instead of panicking in the
                                // networking thread (which used to kill the whole connection
                                // and make the server selection unresponsive).
                                keep_tail_from = Some(0);
                            } else {
                                let account_id = AccountId::from_bytes(&mut byte_reader).unwrap();
                                events.push(NetworkEvent::AccountId { account_id });
                                read_account_id = false;
                            }
                        }

                        while keep_tail_from.is_none() && !byte_reader.is_empty() {
                            match packet_handler.process_one(&mut byte_reader) {
                                HandlerResult::Ok(packet_events) => events.extend(packet_events.0.into_iter()),
                                HandlerResult::PacketCutOff => {
                                    let packet_start = byte_reader.get_offset();

                                    // A packet that starts at offset zero and fills the whole
                                    // buffer is larger than anything we can parse; drop it to
                                    // avoid getting stuck forever.
                                    let packet_larger_than_buffer = packet_start == 0 && data_len == buffer.len();

                                    if !packet_larger_than_buffer {
                                        // This is normal TCP fragmentation: remember where the
                                        // packet starts so that its bytes survive until the next
                                        // read appends the rest. Dropping them would leave the
                                        // parser in the middle of a packet and desynchronize the
                                        // whole stream (which used to silently swallow e.g. a
                                        // whole map-load entity spawn burst).
                                        keep_tail_from = Some(packet_start);
                                    }

                                    break;
                                }
                                // An unknown packet does not have to cost us the rest of the
                                // read. Many server packets are variable length and carry their
                                // total length in bytes 2..4; if the length is plausible and the
                                // bytes it points at start another known packet, skip exactly
                                // that many bytes and keep parsing. Only if that is not possible
                                // fall back to discarding the remainder (a fixed-size packet
                                // has no reliable length field, so skipping it could put the
                                // parser in the middle of the next packet).
                                HandlerResult::UnhandledPacket | HandlerResult::InternalError(..) => {
                                    let packet_start = byte_reader.get_offset();

                                    match Self::skip_length_of_unknown_packet(&buffer[..data_len], packet_start, &packet_handler) {
                                        Some(packet_end) => {
                                            let packet_header =
                                                u16::from_le_bytes([buffer[packet_start], buffer[packet_start + 1]]);

                                            #[cfg(feature = "interface")]
                                            korangar_interface::log_debug!(
                                                "networking: skipping unhandled packet 0x{:04X} ({} bytes)",
                                                packet_header,
                                                packet_end - packet_start
                                            );

                                            let skipped = packet_end - packet_start;
                                            let _ = byte_reader.slice::<Self>(skipped);
                                        }
                                        None => {
                                            discard_remainder = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    cut_off_buffer_base = match (keep_tail_from, discard_remainder) {
                        (_, true) => 0,
                        (Some(packet_start), false) => {
                            buffer.copy_within(packet_start..data_len, 0);
                            data_len - packet_start
                        }
                        (None, false) => 0,
                    };

                    for event in events.drain(..) {
                        if let NetworkEvent::UpdateClientTick {client_tick,received_at} = &event && let Ok(mut time_synchronization) = time_synchronization.lock() {
                            time_synchronization.estimated_client_tick(client_tick.0, *received_at);
                        }

                        event_sender.send(event).map_err(|_| NetworkTaskError::ConnectionClosed)?;
                    }
                }
                // Send a keep-alive packet to the server.
                _ = interval.tick() => {
                    ping_factory(&time_synchronization).packet_to_bytes(&mut byte_writer).unwrap();
                    stream.write_all(byte_writer.as_slice()).await.map_err(|_| NetworkTaskError::ConnectionClosed)?;
                    byte_writer.clear();
                }
            }
        }
    }

    /// Returns the offset after the packet starting at `packet_start` when it
    /// is an unknown packet that can be skipped safely.
    ///
    /// Only variable length packets carry their total size in bytes 2..4, so a
    /// fixed-size unknown packet would look like a packet with a random length.
    /// To stay safe the length is only accepted when its size is plausible
    /// (`>= 4`) and the resulting offset either is the end of the read or
    /// points at the header of another registered packet.
    fn skip_length_of_unknown_packet(
        data: &[u8],
        packet_start: usize,
        packet_handler: &PacketHandler<NetworkEventList, Callback>,
    ) -> Option<usize> {
        if packet_start + 4 > data.len() {
            return None;
        }

        let packet_length = u16::from_le_bytes([data[packet_start + 2], data[packet_start + 3]]) as usize;

        if packet_length < 4 {
            return None;
        }

        let packet_end = packet_start.checked_add(packet_length)?;

        if packet_end > data.len() {
            return None;
        }

        if packet_end == data.len() {
            return Some(packet_end);
        }

        if packet_end + 2 > data.len() {
            return None;
        }

        let next_header = u16::from_le_bytes([data[packet_end], data[packet_end + 1]]);

        match packet_handler.is_registered(PacketHeader(next_header)) {
            true => Some(packet_end),
            false => None,
        }
    }

    pub fn connect_to_login_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        address: SocketAddr,
        username: impl Into<String>,
        password: impl Into<String>,
    ) {
        // A previous connection can still be registered (for example when the player
        // clicks the server button twice or the server rejected the previous session).
        // Drop it and start a fresh connection instead of silently ignoring the
        // request, which used to make the character/server selection unresponsive.
        let _ = self.login_server_connection.take();

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        if self
            .command_sender
            .send(ServerConnectCommand::Login {
                address,
                action_receiver,
                event_sender,
                packet_version,
            })
            .is_err()
        {
            // The networking thread is gone; fail gracefully instead of panicking.
            self.login_server_connection = ServerConnection::Disconnected;
            return;
        }

        let login_packet = LoginServerLoginPacket::new(username.into(), password.into());

        self.packet_callback.outgoing_packet(&login_packet);

        let mut byte_writer = ByteWriter::with_encoding(UTF_8);
        login_packet.packet_to_bytes(&mut byte_writer).unwrap();
        if action_sender.send(byte_writer.into_inner()).is_err() {
            // The connection task failed before it could accept the login packet
            // (e.g. a transient connection error). Fail gracefully instead of panicking.
            self.login_server_connection = ServerConnection::Disconnected;
            return;
        }

        self.login_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
            packet_version,
        };
    }

    pub fn connect_to_character_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        login_data: &LoginServerLoginData,
        server: CharacterServerInformation,
    ) {
        // A previous connection can still be registered (for example when the player
        // clicks the server button twice or the server rejected the previous session).
        // Drop it and start a fresh connection instead of silently ignoring the
        // request, which used to make the character/server selection unresponsive.
        let _ = self.character_server_connection.take();

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let address = SocketAddr::new(IpAddr::V4(server.server_ip.into()), server.server_port);

        if self
            .command_sender
            .send(ServerConnectCommand::Character {
                address,
                action_receiver,
                event_sender,
                packet_version,
            })
            .is_err()
        {
            // The networking thread is gone; fail gracefully instead of panicking.
            self.character_server_connection = ServerConnection::Disconnected;
            return;
        }

        let login_packet = CharacterServerLoginPacket::new(
            login_data.account_id,
            login_data.login_id1,
            login_data.login_id2,
            login_data.sex,
        );

        self.packet_callback.outgoing_packet(&login_packet);

        let mut byte_writer = ByteWriter::with_encoding(UTF_8);
        login_packet.packet_to_bytes(&mut byte_writer).unwrap();
        if action_sender.send(byte_writer.into_inner()).is_err() {
            // The character server connection task dropped before accepting the login
            // packet (e.g. transient connection error); fail gracefully, don't panic.
            self.character_server_connection = ServerConnection::Disconnected;
            return;
        }

        self.character_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
            packet_version,
        };
    }

    pub fn connect_to_map_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        login_server_login_data: &LoginServerLoginData,
        character_server_login_data: CharacterServerLoginData,
    ) {
        // A previous connection can still be registered (for example when the player
        // clicks the server button twice or the server rejected the previous session).
        // Drop it and start a fresh connection instead of silently ignoring the
        // request, which used to make the character/server selection unresponsive.
        let _ = self.map_server_connection.take();

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let address = SocketAddr::new(character_server_login_data.server_ip, character_server_login_data.server_port);

        if self
            .command_sender
            .send(ServerConnectCommand::Map {
                address,
                action_receiver,
                event_sender,
                packet_version,
            })
            .is_err()
        {
            // The networking thread is gone; fail gracefully instead of panicking.
            self.map_server_connection = ServerConnection::Disconnected;
            return;
        }

        let login_packet = MapServerLoginPacket::new(
            login_server_login_data.account_id,
            character_server_login_data.character_id,
            login_server_login_data.login_id1,
            // Always passing 100 seems to work fine for now, but it might cause
            // issues when connecting to something other than rAthena.
            ClientTick(100),
            login_server_login_data.sex,
        );

        self.packet_callback.outgoing_packet(&login_packet);

        let mut byte_writer = ByteWriter::with_encoding(UTF_8);
        login_packet.packet_to_bytes(&mut byte_writer).unwrap();
        if action_sender.send(byte_writer.into_inner()).is_err() {
            // The map server connection task dropped before accepting the login packet
            // (e.g. transient connection error); fail gracefully, don't panic.
            self.map_server_connection = ServerConnection::Disconnected;
            return;
        }

        self.map_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
            packet_version,
        };
    }

    /// Requests a manual close of the login server connection.
    ///
    /// Only a live connection is closed. Marking an already disconnected
    /// connection as "closing manually" used to make the connection handler
    /// emit a spurious disconnect event, which (for the map server) sent the
    /// client back to an empty character selection right after login.
    pub fn disconnect_from_login_server(&mut self) {
        if matches!(self.login_server_connection, ServerConnection::Connected { .. }) {
            self.login_server_connection = ServerConnection::ClosingManually;
        }
    }

    /// See [`Self::disconnect_from_login_server`].
    pub fn disconnect_from_character_server(&mut self) {
        if matches!(self.character_server_connection, ServerConnection::Connected { .. }) {
            self.character_server_connection = ServerConnection::ClosingManually;
        }
    }

    /// See [`Self::disconnect_from_login_server`].
    pub fn disconnect_from_map_server(&mut self) {
        if matches!(self.map_server_connection, ServerConnection::Connected { .. }) {
            self.map_server_connection = ServerConnection::ClosingManually;
        }
    }

    pub fn is_login_server_connected(&self) -> bool {
        matches!(self.login_server_connection, ServerConnection::Connected { .. })
    }

    pub fn is_character_server_connected(&self) -> bool {
        matches!(self.character_server_connection, ServerConnection::Connected { .. })
    }

    pub fn is_map_server_connected(&self) -> bool {
        matches!(self.map_server_connection, ServerConnection::Connected { .. })
    }

    fn character_server_packet_version(&self) -> Result<SupportedPacketVersion, NotConnectedError> {
        match &self.character_server_connection {
            ServerConnection::Connected { packet_version, .. } => Ok(*packet_version),
            _ => Err(NotConnectedError),
        }
    }

    fn map_server_packet_version(&self) -> Result<SupportedPacketVersion, NotConnectedError> {
        match &self.map_server_connection {
            ServerConnection::Connected { packet_version, .. } => Ok(*packet_version),
            _ => Err(NotConnectedError),
        }
    }

    fn send_character_server_packet(&mut self, packet: impl CharacterServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.character_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(&packet);

                // FIX: Don't unwrap.
                let mut byte_writer = ByteWriter::with_encoding(UTF_8);
                packet.packet_to_bytes(&mut byte_writer).unwrap();
                action_sender.send(byte_writer.into_inner()).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    fn send_map_server_packet(&mut self, packet: impl MapServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.map_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(&packet);

                // FIX: Don't unwrap.
                let mut byte_writer = ByteWriter::with_encoding(UTF_8);
                packet.packet_to_bytes(&mut byte_writer).unwrap();
                action_sender.send(byte_writer.into_inner()).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    fn create_login_server_packet_handler(
        packet_callback: Callback,
        packet_version: SupportedPacketVersion,
    ) -> Result<PacketHandler<NetworkEventList, Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, Callback>::with_callback(packet_callback);

        match packet_version {
            SupportedPacketVersion::_20220406 => packet_versions::version_20220406::register_login_server_packets(&mut packet_handler)?,
        }

        Ok(packet_handler)
    }

    fn create_character_server_packet_handler(
        packet_callback: Callback,
        packet_version: SupportedPacketVersion,
    ) -> Result<PacketHandler<NetworkEventList, Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, Callback>::with_callback(packet_callback);

        match packet_version {
            SupportedPacketVersion::_20220406 => packet_versions::version_20220406::register_character_server_packets(&mut packet_handler)?,
        }

        Ok(packet_handler)
    }

    fn create_map_server_packet_handler(
        packet_callback: Callback,
        packet_version: SupportedPacketVersion,
    ) -> Result<PacketHandler<NetworkEventList, Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, Callback>::with_callback(packet_callback);

        match packet_version {
            SupportedPacketVersion::_20220406 => packet_versions::version_20220406::register_map_server_packets(&mut packet_handler)?,
        }

        Ok(packet_handler)
    }

    pub fn request_character_list(&mut self) -> Result<(), NotConnectedError> {
        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(RequestCharacterListPacket::default()),
        }
    }

    pub fn select_character(&mut self, character_slot: usize) -> Result<(), NotConnectedError> {
        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(SelectCharacterPacket::new(character_slot as u8)),
        }
    }

    pub fn create_character(&mut self, slot: usize, name: String) -> Result<(), NotConnectedError> {
        let hair_color = 0;
        let hair_style = 0;
        let start_job_id = JobId(0);
        let sex = Sex::Male;

        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(CreateCharacterPacket::new(
                name,
                slot as u8,
                hair_color,
                hair_style,
                start_job_id,
                sex,
            )),
        }
    }

    pub fn delete_character(&mut self, character_id: CharacterId) -> Result<(), NotConnectedError> {
        let email = "a@a.com".to_string();

        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(DeleteCharacterPacket::new(character_id, email)),
        }
    }

    pub fn switch_character_slot(&mut self, origin_slot: usize, destination_slot: usize) -> Result<(), NotConnectedError> {
        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_character_server_packet(SwitchCharacterSlotPacket::new(origin_slot as u16, destination_slot as u16))
            }
        }
    }

    pub fn map_loaded(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MapLoadedPacket::default()),
        }
    }

    pub fn request_client_tick(&mut self) -> Result<(), NotConnectedError> {
        let client_tick = self
            .time_synchronization
            .lock()
            .map(|time_synchronization| time_synchronization.client_tick as u32)
            .unwrap_or(100);

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestServerTickPacket::new(ClientTick(client_tick))),
        }
    }

    pub fn respawn(&mut self) -> Result<(), NotConnectedError> {
        let result = match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RestartPacket::new(RestartType::Respawn)),
        };

        if result.is_ok() {
            self.last_restart = Some((RestartType::Respawn, Instant::now()));
        }

        result
    }

    pub fn log_out(&mut self) -> Result<(), NotConnectedError> {
        let result = match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RestartPacket::new(RestartType::Disconnect)),
        };

        if result.is_ok() {
            self.last_restart = Some((RestartType::Disconnect, Instant::now()));
        }

        result
    }

    /// Ask the map server for a clean disconnect (`CZ_REQ_DISCONNECT`, 0x018A).
    ///
    /// Used when the client is closed while in game so the map server saves the
    /// character and reports it offline to the character/login server. Without
    /// this the account can stay "online" for a while and an immediate
    /// relogin is rejected as already online.
    pub fn request_disconnect(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestDisconnectPacket::new(0)),
        }
    }

    pub fn player_move(&mut self, position: WorldPosition) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestPlayerMovePacket::new(position)),
        }
    }

    pub fn warp_to_map(&mut self, map_name: String, position: TilePosition) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestWarpToMapPacket::new(map_name, position)),
        }
    }

    pub fn entity_details(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestDetailsPacket::new(entity_id)),
        }
    }

    /// Requests the emblem image of a guild (`CZ_REQ_GUILD_EMBLEM_IMG`,
    /// `0x0151`).
    pub fn request_guild_emblem(&mut self, guild_id: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildEmblemRequestPacket::new(guild_id)),
        }
    }

    pub fn player_attack(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestActionPacket::new(entity_id, Action::Attack)),
        }
    }

    pub fn send_emotion(&mut self, emotion: u8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestEmotionPacket::new(emotion)),
        }
    }

    pub fn pick_up_item(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ItemPickupRequestPacket::new(entity_id)),
        }
    }

    pub fn send_chat_message(&mut self, player_name: &str, text: &str) -> Result<(), NotConnectedError> {
        let message = format!("{} : {}", player_name, text);

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GlobalMessagePacket::new(message)),
        }
    }

    /// Whisper a message to another character.
    pub fn send_whisper(&mut self, target_name: &str, message: &str) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(WhisperPacket::new(target_name.to_owned(), message.to_owned()))
            }
        }
    }

    pub fn start_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(StartDialogPacket::new(npc_id)),
        }
    }

    pub fn next_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(NextDialogPacket::new(npc_id)),
        }
    }

    pub fn close_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CloseDialogPacket::new(npc_id)),
        }
    }

    pub fn choose_dialog_option(&mut self, npc_id: EntityId, option: i8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ChooseDialogOptionPacket::new(npc_id, option)),
        }
    }

    /// Use an item from the inventory (potions, usable items, ...).
    pub fn use_item(&mut self, item_index: InventoryIndex, item_id: ItemId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(UseItemPacket::new(item_index, item_id)),
        }
    }

    pub fn request_item_equip(&mut self, item_index: InventoryIndex, equip_position: EquipPosition) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestEquipItemPacket::new(item_index, equip_position)),
        }
    }

    pub fn request_item_unequip(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestUnequipItemPacket::new(item_index)),
        }
    }

    pub fn cast_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(UseSkillAtTargetPacket2019::new(skill_id, skill_level, entity_id))
            }
        }
    }

    pub fn cast_ground_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        target_position: TilePosition,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(UseSkillOnGroundPacket::new(skill_level, skill_id, target_position))
            }
        }
    }

    pub fn cast_channeling_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        entity_id: EntityId,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(StartUseSkillPacket::new(skill_id, skill_level, entity_id)),
        }
    }

    pub fn stop_channeling_skill(&mut self, skill_id: SkillId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(EndUseSkillPacket::new(skill_id)),
        }
    }

    pub fn add_friend(&mut self, name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(AddFriendPacket::new(name)),
        }
    }

    pub fn remove_friend(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RemoveFriendPacket::new(account_id, character_id)),
        }
    }

    pub fn reject_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(FriendRequestResponsePacket::new(
                account_id,
                character_id,
                FriendRequestResponse::Reject,
            )),
        }
    }

    pub fn accept_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(FriendRequestResponsePacket::new(
                account_id,
                character_id,
                FriendRequestResponse::Accept,
            )),
        }
    }

    pub fn set_hotkey_data(&mut self, tab: HotbarTab, index: HotbarSlot, hotkey_data: HotkeyData) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SetHotkeyData2Packet::new(tab, index, hotkey_data)),
        }
    }

    pub fn select_buy_or_sell(&mut self, shop_id: ShopId, buy_or_sell: BuyOrSellOption) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SelectBuyOrSellPacket::new(shop_id, buy_or_sell)),
        }
    }

    pub fn purchase_items(&mut self, items: Vec<ShopItem<u32>>) -> Result<(), NotConnectedError> {
        let item_information = items
            .into_iter()
            .map(|item| BuyShopItemInformation {
                item_id: item.item_id,
                amount: item.metadata,
            })
            .collect();

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BuyShopItemsPacket::new(item_information)),
        }
    }

    pub fn close_shop(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CloseShopPacket::new()),
        }
    }

    pub fn sell_items(&mut self, items: Vec<SoldItemInformation>) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SellItemsPacket { items }),
        }
    }

    pub fn request_stat_up(&mut self, stat_type: StatUpType) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestStatUpPacket::new(stat_type)),
        }
    }

    pub fn level_up_skill(&mut self, skill_id: SkillId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(LevelUpSkillPacket::new(skill_id)),
        }
    }
}

#[cfg(test)]
mod packet_handlers {
    use ragnarok_bytes::{ByteReader, ByteWriter};
    use ragnarok_packets::handler::{HandlerResult, NoPacketCallback};
    use ragnarok_packets::{
        AccountId, CharacterId, DisplayEmotionPacket, EntityId, GuildEmblemImagePacket, GuildEmblemRequestPacket, LoginFailedPacket2,
        LoginFailedReason2, PacketExt, RequestEmotionPacket, RequestEntityDetailsSuccessPacket, RequestPlayerDetailsSuccessPacket,
    };

    use crate::{NetworkEvent, NetworkingSystem, SupportedPacketVersion, UnifiedLoginFailedReason};

    #[test]
    fn login_server() {
        let result = NetworkingSystem::create_login_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406);
        assert!(result.is_ok());
    }

    #[test]
    fn character_server() {
        let result = NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406);
        assert!(result.is_ok());
    }

    #[test]
    fn map_server() {
        let result = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406);
        assert!(result.is_ok());
    }

    #[test]
    fn emotion_request_uses_expected_header() {
        let mut writer = ByteWriter::new();
        RequestEmotionPacket { emotion: 3 }.packet_to_bytes(&mut writer).unwrap();

        assert_eq!(writer.as_slice(), &[0xBF, 0x00, 0x03]);
    }

    #[test]
    fn guild_emblem_request_uses_expected_header() {
        let mut writer = ByteWriter::new();
        GuildEmblemRequestPacket::new(1).packet_to_bytes(&mut writer).unwrap();

        // `0x0151` + guild id (u32 little endian).
        assert_eq!(writer.as_slice(), &[0x51, 0x01, 0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn sound_packet_is_parsed_into_play_sound_event() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // `ZC_SOUND` (0x01D3), 2019 layout: header + name[24] + act + term + aid.
        let mut bytes = vec![0xD3, 0x01];
        let mut name = [0u8; 24];
        name[..18].copy_from_slice(b"effect\\ef_blessing");
        bytes.extend_from_slice(&name);
        bytes.push(1);
        bytes.extend_from_slice(&500u32.to_le_bytes());
        bytes.extend_from_slice(&0x1234_5678u32.to_le_bytes());
        assert_eq!(bytes.len(), 35);

        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("sound packet was not handled");
        };

        assert!(matches!(
            events.0.as_slice(),
            [NetworkEvent::PlaySound {
                name,
                act: 1,
                term: 500,
                account_id: AccountId(0x1234_5678),
            }] if name == "effect\\ef_blessing"
        ));
    }

    #[test]
    fn guild_emblem_image_packet_is_parsed() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let emblem_data = vec![0x42, 0x4D, 0x36, 0x06, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];
        let packet = GuildEmblemImagePacket {
            guild_id: 1,
            emblem_id: 3,
            emblem_data: emblem_data.clone(),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();

        // `0x0152` + packet length + guild id + emblem id + BMP data.
        assert_eq!(writer.as_slice().len(), 12 + emblem_data.len());
        assert_eq!(&writer.as_slice()[..2], &[0x52, 0x01]);
        assert_eq!(&writer.as_slice()[2..4], &(12u16 + emblem_data.len() as u16).to_le_bytes());

        let mut reader = ByteReader::without_metadata(writer.as_slice());

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("guild emblem packet was not handled");
        };

        assert!(matches!(
            events.0.as_slice(),
            [NetworkEvent::GuildEmblemImage {
                guild_id: 1,
                emblem_id: 3,
                emblem_data: data,
            }] if data == &emblem_data
        ));
    }

    #[test]
    fn player_details_packet_carries_guild_name() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let packet = RequestPlayerDetailsSuccessPacket::new(
            CharacterId(0x1234_5678),
            "VerifyBot".to_owned(),
            String::new(),
            "Domination".to_owned(),
            String::new(),
            0,
        );

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        let mut reader = ByteReader::without_metadata(writer.as_slice());

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("player-details packet was not handled");
        };

        assert!(matches!(
            events.0.as_slice(),
            [NetworkEvent::UpdateEntityDetails {
                entity_id: EntityId(0x1234_5678),
                guild_name: Some(guild_name),
                ..
            }] if guild_name == "Domination"
        ));
    }

    #[test]
    fn player_details_packet_without_guild_has_no_guild_name() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let packet = RequestPlayerDetailsSuccessPacket::new(
            CharacterId(0x1111_2222),
            "VerifyBot".to_owned(),
            String::new(),
            // rAthena pads "no guild" with an empty fixed-length string.
            String::new(),
            String::new(),
            0,
        );

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        let mut reader = ByteReader::without_metadata(writer.as_slice());

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("player-details packet was not handled");
        };

        assert!(matches!(events.0.as_slice(), [NetworkEvent::UpdateEntityDetails {
            entity_id: EntityId(0x1111_2222),
            guild_name: None,
            ..
        }]));
    }

    #[test]
    fn npc_details_packet_has_no_guild_name() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let packet = RequestEntityDetailsSuccessPacket::new(EntityId(0x3333_4444), 0, "Prontera Guard".to_owned(), String::new());

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        let mut reader = ByteReader::without_metadata(writer.as_slice());

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("entity-details packet was not handled");
        };

        assert!(matches!(events.0.as_slice(), [NetworkEvent::UpdateEntityDetails {
            entity_id: EntityId(0x3333_4444),
            guild_name: None,
            ..
        }]));
    }

    #[test]
    fn longlong_stat_update_packet_carries_next_base_experience() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // 0x0ACB (ZC_LONGLONGPAR_CHANGE): varId 22 (SP_NEXTBASEEXP), 10 as i64.
        let packet: [u8; 12] = [
            0xCB, 0x0A, // header
            22, 0x00, // varId
            10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // amount
        ];
        let mut reader = ByteReader::without_metadata(&packet);

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("longlong stat update packet was not handled");
        };

        assert!(matches!(events.0.as_slice(), [NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::NextBaseExperience(10),
        }]));
    }

    #[test]
    fn display_emotion_packet_preserves_entity_and_emotion() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
        let packet = DisplayEmotionPacket {
            entity_id: EntityId(0x1234_5678),
            emotion: 3,
        };
        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        let mut reader = ByteReader::without_metadata(writer.as_slice());

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("display-emotion packet was not handled");
        };

        assert!(matches!(events.0.as_slice(), [NetworkEvent::DisplayEmotion {
            entity_id: EntityId(0x1234_5678),
            emotion: 3,
        }]));
    }
}
