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
pub use self::event::{DisconnectReason, NetworkEvent, PartyMemberDetails};
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

/// What to do with a packet that has no registered handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnknownPacketAction {
    /// Skip exactly this many bytes and continue parsing.
    Skip(usize),
    /// The packet is (partially) known but not complete in this read yet; keep
    /// the bytes for the next read.
    NeedMoreData,
    /// The stream cannot be resynchronized; discard the rest of the read.
    Discard,
}

/// Decide how to handle an unhandled packet at `packet_start`.
///
/// The committed 2019 opcode length registry
/// (`ragnarok_packets::opcode_lengths_2019`) is authoritative: for a known
/// fixed-size packet the exact number of bytes is skipped, for a known
/// variable-length packet its declared length field is used. Packets that are
/// not in the registry fall back to the previous conservative heuristic (accept
/// the length field only if it lands on another registered header or the end of
/// the read) so unknown opcodes never lose more of the stream than before.
fn unknown_packet_action<Callback>(
    data: &[u8],
    packet_start: usize,
    packet_handler: &PacketHandler<NetworkEventList, Callback>,
) -> UnknownPacketAction
where
    Callback: PacketCallback,
{
    if packet_start + 2 > data.len() {
        return UnknownPacketAction::NeedMoreData;
    }

    let header = PacketHeader(u16::from_le_bytes([data[packet_start], data[packet_start + 1]]));

    match ragnarok_packets::opcode_lengths_2019::opcode_length(header) {
        Some(ragnarok_packets::opcode_lengths_2019::OpcodeLength::Fixed(packet_length)) => {
            match packet_start.checked_add(packet_length) {
                Some(packet_end) if packet_end <= data.len() => UnknownPacketAction::Skip(packet_length),
                // The registry knows the size, so additional reads can complete
                // the packet instead of throwing the rest of the stream away.
                _ => UnknownPacketAction::NeedMoreData,
            }
        }
        Some(ragnarok_packets::opcode_lengths_2019::OpcodeLength::Variable { base_size }) => {
            let packet_length = declared_length(data, packet_start);

            match packet_length {
                // Sanity check the length field before trusting it: it has to
                // be at least the fixed part of the packet according to the
                // registry. A registry entry that turns out to be wrong falls
                // back to the heuristic instead of corrupting the stream.
                Some(packet_length) if packet_length >= 4 && packet_length >= base_size => match packet_start.checked_add(packet_length) {
                    Some(packet_end) if packet_end <= data.len() => UnknownPacketAction::Skip(packet_length),
                    _ => UnknownPacketAction::NeedMoreData,
                },
                _ => unknown_packet_heuristic(data, packet_start, packet_handler),
            }
        }
        None => unknown_packet_heuristic(data, packet_start, packet_handler),
    }
}

fn declared_length(data: &[u8], packet_start: usize) -> Option<usize> {
    if packet_start + 4 > data.len() {
        return None;
    }

    Some(u16::from_le_bytes([data[packet_start + 2], data[packet_start + 3]]) as usize)
}

/// Conservative fallback for opcodes that are not in the 2019 registry.
///
/// Only variable length packets carry their total size in bytes 2..4, so a
/// fixed-size unknown packet would look like a packet with a random length. To
/// stay safe the length is only accepted when its size is plausible (`>= 4`)
/// and the resulting offset either is the end of the read or points at the
/// header of another registered packet.
fn unknown_packet_heuristic<Callback>(
    data: &[u8],
    packet_start: usize,
    packet_handler: &PacketHandler<NetworkEventList, Callback>,
) -> UnknownPacketAction
where
    Callback: PacketCallback,
{
    let Some(packet_length) = declared_length(data, packet_start) else {
        return UnknownPacketAction::Discard;
    };

    if packet_length < 4 {
        return UnknownPacketAction::Discard;
    }

    let Some(packet_end) = packet_start.checked_add(packet_length) else {
        return UnknownPacketAction::Discard;
    };

    if packet_end > data.len() {
        return UnknownPacketAction::Discard;
    }

    if packet_end == data.len() {
        return UnknownPacketAction::Skip(packet_length);
    }

    if packet_end + 2 > data.len() {
        return UnknownPacketAction::Discard;
    }

    let next_header = u16::from_le_bytes([data[packet_end], data[packet_end + 1]]);

    match packet_handler.is_registered(PacketHeader(next_header)) {
        true => UnknownPacketAction::Skip(packet_length),
        false => UnknownPacketAction::Discard,
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

                                    match unknown_packet_action(&buffer[..data_len], packet_start, &packet_handler) {
                                        UnknownPacketAction::Skip(skipped) => {
                                            // The 2019 registry (or, for opcodes that are
                                            // not in it, the conservative heuristic) knows
                                            // exactly how many bytes this packet occupies;
                                            // skipping them keeps the rest of the read alive.
                                            let _ = byte_reader.slice::<Self>(skipped);
                                        }
                                        UnknownPacketAction::NeedMoreData => {
                                            // A known packet is not complete yet. Keep the
                                            // bytes for the next read instead of discarding
                                            // them. If the buffer is already full the packet
                                            // can never complete, so drop the read instead of
                                            // getting stuck.
                                            if packet_start == 0 && data_len == buffer.len() {
                                                discard_remainder = true;
                                            } else {
                                                keep_tail_from = Some(packet_start);
                                            }

                                            break;
                                        }
                                        UnknownPacketAction::Discard => {
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
                    if ping_factory(&time_synchronization).packet_to_bytes(&mut byte_writer).is_err() {
                        eprintln!("[network] failed to serialise ping packet; skipping keep-alive");
                        byte_writer.clear();
                        continue;
                    }
                    stream.write_all(byte_writer.as_slice()).await.map_err(|_| NetworkTaskError::ConnectionClosed)?;
                    byte_writer.clear();
                }
            }
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
        if login_packet.packet_to_bytes(&mut byte_writer).is_err() {
            // Packet serialisation of a well-formed login packet is not expected to
            // fail; if it does, fail the connection instead of panicking.
            self.login_server_connection = ServerConnection::Disconnected;
            return;
        }
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
        if login_packet.packet_to_bytes(&mut byte_writer).is_err() {
            self.character_server_connection = ServerConnection::Disconnected;
            return;
        }
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
        if login_packet.packet_to_bytes(&mut byte_writer).is_err() {
            self.map_server_connection = ServerConnection::Disconnected;
            return;
        }
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

                let mut byte_writer = ByteWriter::with_encoding(UTF_8);
                packet.packet_to_bytes(&mut byte_writer).map_err(|_| NotConnectedError)?;
                action_sender.send(byte_writer.into_inner()).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    fn send_map_server_packet(&mut self, packet: impl MapServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.map_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(&packet);

                let mut byte_writer = ByteWriter::with_encoding(UTF_8);
                packet.packet_to_bytes(&mut byte_writer).map_err(|_| NotConnectedError)?;
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

    /// Delete a character.
    ///
    /// The CondaRO character server runs with `char_del_option: 1`, i.e. it
    /// requires the account e-mail address as the deletion confirmation code
    /// (rAthena `CHAR_DEL_EMAIL`). Sending a hardcoded placeholder made every
    /// deletion request fail. The login server never sends the address to the
    /// client, so the user has to type it in.
    pub fn delete_character(&mut self, character_id: CharacterId, email: String) -> Result<(), NotConnectedError> {
        // The wire field is a fixed 40-byte C string. Keeping one byte for the
        // terminator makes sure the server side `stricmp` sees a terminated
        // string even for the maximum length address.
        let email = email.trim().chars().take(39).collect::<String>();

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

    /// Move items from the inventory into the open Kafra storage
    /// (`CZ_MOVE_ITEM_FROM_BODY_TO_STORE`, 2019 opcode 0x00F3).
    pub fn storage_deposit(&mut self, inventory_index: InventoryIndex, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(StorageDepositPacket { inventory_index, amount }),
        }
    }

    /// Move items from the open Kafra storage back into the inventory
    /// (`CZ_MOVE_ITEM_FROM_STORE_TO_BODY`, 2019 opcode 0x00F5). The server
    /// expects the storage slot encoded as `slot + 1`.
    pub fn storage_withdraw(&mut self, storage_index: u16, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(StorageWithdrawPacket {
                storage_index: storage_index.saturating_add(1),
                amount,
            }),
        }
    }

    /// Close the open Kafra storage window (`CZ_CLOSE_STORE`, 2019 opcode
    /// 0x00F7).
    pub fn close_storage(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CloseStoragePacket {}),
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
                self.send_map_server_packet(UseSkillAtTargetPacket2019::new(skill_level, skill_id, entity_id))
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

    /// Create a party with the given name (`CZ_MAKE_GROUP`, 0x00F9).
    pub fn create_party(&mut self, name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CreatePartyPacket { party_name: name }),
        }
    }

    /// Invite a character to the current party by name (`CZ_PARTY_JOIN_REQ`,
    /// 0x02C4).
    pub fn invite_to_party(&mut self, name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyInviteRequestPacket2019 { character_name: name }),
        }
    }

    /// Accept or reject a party invitation (`CZ_PARTY_JOIN_REQ_ACK`, 0x02C7).
    pub fn reply_party_invite(&mut self, party_id: PartyId, accept: bool) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyInviteReplyRequestPacket2019 {
                party_id,
                accept: accept as u8,
            }),
        }
    }

    /// Leave the current party (`CZ_REQ_LEAVE_GROUP`, 0x0100).
    pub fn leave_party(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(LeavePartyPacket {}),
        }
    }

    /// Expel a party member (leader only, `CZ_REQ_EXPEL_GROUP_MEMBER`, 0x0103).
    pub fn expel_party_member(&mut self, account_id: AccountId, name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ExpelPartyMemberPacket {
                account_id,
                character_name: name,
            }),
        }
    }

    /// Send a party chat message (`CZ_REQUEST_CHAT_PARTY`, 0x0108).
    ///
    /// Like the regular chat the server expects the `<name> : <message>`
    /// format.
    pub fn send_party_chat_message(&mut self, player_name: &str, text: &str) -> Result<(), NotConnectedError> {
        let message = format!("{} : {}", player_name, text);

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyChatRequestPacket { message }),
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
        // The 2019 CZ_PC_PURCHASE_ITEMLIST uses `{ amount u16, item id u32 }` entries.
        let item_information = items
            .into_iter()
            .map(|item| BuyItemInformation {
                item_id: item.item_id,
                amount: item.metadata.min(u16::MAX as u32) as u16,
            })
            .collect();

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BuyItemsPacket { items: item_information }),
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
mod contract_2019;

#[cfg(test)]
mod packet_handlers {
    use std::collections::HashSet;

    use ragnarok_bytes::{ByteReader, ByteWriter};
    use ragnarok_packets::handler::{HandlerResult, NoPacketCallback};
    use ragnarok_packets::{
        AccountId, AttackRange, CharacterId, ClientTick, Direction, DisplayEmotionPacket, EntityId, GuildEmblemImagePacket,
        GuildEmblemRequestPacket, MarkerType, PacketExt, PacketHeader, RequestEmotionPacket, RequestEntityDetailsSuccessPacket,
        RequestPlayerDetailsSuccessPacket, SkillId, TilePosition,
    };

    use crate::event::NetworkEventList;
    use crate::packet_versions::noop_policy::{IGNORED_NOOPS, IgnoreClass, NoopServer, register_ignored_noop};
    use crate::{NetworkEvent, NetworkingSystem, SupportedPacketVersion, UnknownPacketAction, unknown_packet_action};

    #[test]
    fn every_ignored_noop_is_registered_with_a_reason() {
        let map_handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
        let character_handler =
            NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let mut seen = HashSet::new();

        for entry in IGNORED_NOOPS {
            assert!(
                seen.insert(entry.header),
                "noop 0x{:04X} is classified more than once",
                entry.header
            );
            assert!(!entry.reason.is_empty(), "noop 0x{:04X} has no reason", entry.header);

            if let IgnoreClass::OwnedElsewhere(owner) = entry.class {
                assert!(!owner.is_empty(), "owned noop 0x{:04X} has no owner", entry.header);
            }

            let headers = match entry.server {
                NoopServer::Map => map_handler.registered_headers(),
                NoopServer::Character => character_handler.registered_headers(),
            };

            assert!(
                headers.contains(&PacketHeader(entry.header)),
                "classified noop 0x{:04X} is not registered for {:?}",
                entry.header,
                entry.server
            );
        }
    }

    #[test]
    fn version_module_registers_no_bare_noops() {
        let source = include_str!("packet_versions/version_20220406.rs");

        assert_eq!(
            source.matches("register_noop::<").count(),
            0,
            "every parsed-but-unconsumed packet must use register_ignored_noop with a classification"
        );
        assert!(
            source.matches("register_ignored_noop::<").count() >= 30,
            "expected the classified noop registrations to remain in place"
        );
    }

    #[test]
    fn dead_2022_packets_are_not_registered() {
        let map_handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
        let headers = map_handler.registered_headers();

        for dead_header in [0x08E2u16, 0x07FB] {
            assert!(
                !headers.contains(&PacketHeader(dead_header)),
                "dead 2022 packet 0x{dead_header:04X} is still registered"
            );
        }

        let packet_source = include_str!("../../ragnarok-packets/src/lib.rs");
        for dead_struct in ["NavigateToMonsterPacket", "UseSkillSuccessPacket"] {
            assert!(
                !packet_source.contains(&format!("pub struct {dead_struct}")),
                "dead 2022 struct {dead_struct} still exists"
            );
        }
    }

    #[test]
    #[should_panic(expected = "is not classified")]
    fn unclassified_noop_is_rejected() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let _ = register_ignored_noop::<_, ragnarok_packets::MapServerLoginSuccessPacket>(&mut handler);
    }

    #[test]
    fn implemented_noops_emit_typed_events() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // `ZC_SPIRITS` (0x01D0): header + entity id u32 + amount i16.
        let mut bytes = vec![0xD0, 0x01];
        bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        bytes.extend_from_slice(&3i16.to_le_bytes());
        assert_eq!(bytes.len(), 8);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("spirits packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::SpiritOrbs {
            entity_id: EntityId(0x0102_0304),
            amount: 3,
        }]));

        // `ZC_SKILL_POSTDELAY` (0x043D): header + skill id u16 + until u32.
        let mut bytes = vec![0x3D, 0x04];
        bytes.extend_from_slice(&34u16.to_le_bytes());
        bytes.extend_from_slice(&500u32.to_le_bytes());
        assert_eq!(bytes.len(), 8);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("skill cooldown packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::SkillCooldown {
            skill_id: SkillId(34),
            until: ClientTick(500),
        }]));

        // `ZC_ATTACK_RANGE` (0x013A): header + range u16.
        let mut bytes = vec![0x3A, 0x01];
        bytes.extend_from_slice(&7u16.to_le_bytes());
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("attack range packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::AttackRangeChanged {
            attack_range: AttackRange(7),
        }]));

        // `ZC_STOPMOVE` (0x0088): header + entity id u32 + x u16 + y u16.
        let mut bytes = vec![0x88, 0x00];
        bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        bytes.extend_from_slice(&120u16.to_le_bytes());
        bytes.extend_from_slice(&45u16.to_le_bytes());
        assert_eq!(bytes.len(), 10);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("stop move packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::EntityStopped {
            entity_id: EntityId(0x0102_0304),
            position: TilePosition { x: 120, y: 45 },
        }]));

        // `ZC_COMPASS` (0x0144): header + npc id u32 + marker type u32 +
        // position (x u32, y u32) + id u8 + colour RGBA.
        let mut bytes = vec![0x44, 0x01];
        bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&120u32.to_le_bytes());
        bytes.extend_from_slice(&45u32.to_le_bytes());
        bytes.push(2);
        bytes.extend_from_slice(&[10, 20, 30, 255]);
        assert_eq!(bytes.len(), 23);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("marker packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::MinimapMarker {
            npc_id: EntityId(0x0102_0304),
            marker_type: MarkerType::DisplayUntilLeave,
            ..
        }]));

        // `ZC_NOTIFY_MAPPROPERTY2` (0x099B): header + map type u16 + flags u32.
        let mut bytes = vec![0x9B, 0x09];
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&0x0001_0000u32.to_le_bytes());
        assert_eq!(bytes.len(), 8);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("map property packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::MapProperties {
            map_type: 3,
            flags: 0x0001_0000,
        }]));

        // `ZC_NOTIFY_CARTITEM_COUNTINFO` (0x0121).
        let mut bytes = vec![0x21, 0x01];
        bytes.extend_from_slice(&5i16.to_le_bytes());
        bytes.extend_from_slice(&100i16.to_le_bytes());
        bytes.extend_from_slice(&500i32.to_le_bytes());
        bytes.extend_from_slice(&8000i32.to_le_bytes());
        assert_eq!(bytes.len(), 14);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("cart count packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::CartInfo {
            current_count: 5,
            maximum_count: 100,
            current_weight: 500,
            maximum_weight: 8000,
        }]));

        // `ZC_CONFIG_NOTIFY` (0x02DA): header + flag u8.
        let bytes = vec![0xDA, 0x02, 1];
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("show equip packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::ShowEquipmentWindow {
            open: true
        }]));

        // `ZC_AUTH_FAIL` (0x006A): header + reason u8.
        let bytes = vec![0x6A, 0x00, 1];
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("map auth failure packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::MapServerAuthFailed { reason: 1 }]));

        // `ZC_NOTIFY_CRITICALWEIGHT` (0x0ADE): header + percentage u32.
        let mut bytes = vec![0xDE, 0x0A];
        bytes.extend_from_slice(&50u32.to_le_bytes());
        assert_eq!(bytes.len(), 6);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("critical weight packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::CriticalWeight { percentage: 50 }]));

        // `ZC_AUTH_OK_GID` (0x0283): header + account id u32 (classified noop).
        let mut bytes = vec![0x83, 0x02];
        bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        assert_eq!(bytes.len(), 6);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("auth ok gid packet was not handled");
        };
        assert!(events.0.is_empty());

        // `ZC_UPDATE_MAPINFO` (0x0192): header + x u16 + y u16 + cell type u16 +
        // map name[16].
        let mut bytes = vec![0x92, 0x01];
        bytes.extend_from_slice(&120u16.to_le_bytes());
        bytes.extend_from_slice(&45u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        let mut map_name = [0u8; 16];
        map_name[..8].copy_from_slice(b"prontera");
        bytes.extend_from_slice(&map_name);
        assert_eq!(bytes.len(), 24);
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("map cell packet was not handled");
        };
        assert!(matches!(events.0.as_slice(), [NetworkEvent::MapCellChanged {
            position: TilePosition { x: 120, y: 45 },
            cell_type: 1,
            ..
        }]));
    }

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
    fn heal_packet_emits_heal_effect_without_entity_id() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // `ZC_RECOVERY` (0x0A27), 2019 layout: header + type (SP_HP = 5) + amount u32.
        let mut bytes = vec![0x27, 0x0A];
        bytes.extend_from_slice(&5u16.to_le_bytes());
        bytes.extend_from_slice(&123u32.to_le_bytes());
        assert_eq!(bytes.len(), 8);

        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("heal packet was not handled");
        };

        assert!(matches!(events.0.as_slice(), [NetworkEvent::HealEffect {
            entity_id: None,
            heal_amount: 123,
        }]));
    }

    #[test]
    fn state_change_packet_emits_state_change() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // `ZC_STATE_CHANGE` (0x0229), 2019 layout:
        // header + AID u32 + bodyState u16 + healthState u16 + effectState u32 +
        // isPKMode u8.
        let mut bytes = vec![0x29, 0x02];
        bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes()); // sitting? at least a state value
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&0x10u32.to_le_bytes()); // OPTION_FALCON
        bytes.push(1);
        assert_eq!(bytes.len(), 15);

        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("state change packet was not handled");
        };

        assert!(matches!(events.0.as_slice(), [NetworkEvent::StateChange {
            entity_id: EntityId(0x0102_0304),
            body_state: 2,
            health_state: 1,
            effect_state: 0x10,
            is_pk_mode_on: true,
        }]));
    }

    #[test]
    fn change_direction_packet_emits_change_direction() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // `ZC_CHANGE_DIRECTION` (0x009C), 2019 layout: header + srcId u32 + headDir u16
        // + dir u8.
        let mut bytes = vec![0x9C, 0x00];
        bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.push(4);
        assert_eq!(bytes.len(), 9);

        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("change direction packet was not handled");
        };

        // Wire direction 4 maps to client direction `(8 - 4 + 4) & 7 = 0`.
        assert!(matches!(events.0.as_slice(), [NetworkEvent::ChangeDirection {
            entity_id: EntityId(0x0102_0304),
            head_direction: 3,
            direction: Direction::North,
        }]));
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

    /// Builds a 2019 `SKILLDATA` payload (id u16, inf i32, level u16, sp u16,
    /// range u16, name[24], up_flag u8) — the layout the 20190605 server sends
    /// in both `ZC_SKILLINFO_LIST` (0x010F) and `ZC_ADD_SKILL` (0x0111).
    fn skill_data_2019(skill_id: u16, skill_type: i32, level: u16, sp: u16, range: u16, name: &str, upgradable: u8) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&skill_id.to_le_bytes());
        data.extend_from_slice(&skill_type.to_le_bytes());
        data.extend_from_slice(&level.to_le_bytes());
        data.extend_from_slice(&sp.to_le_bytes());
        data.extend_from_slice(&range.to_le_bytes());

        let mut name_bytes = [0u8; 24];
        for (index, byte) in name.as_bytes().iter().take(23).enumerate() {
            name_bytes[index] = *byte;
        }
        data.extend_from_slice(&name_bytes);
        data.push(upgradable);
        data
    }

    #[test]
    fn add_skill_packet_parses_the_2019_skill_data_layout() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0111u16.to_le_bytes());
        packet.extend_from_slice(&skill_data_2019(5, 4, 10, 8, 1, "Crazy Uproar", 1));

        assert_eq!(packet.len(), 39);

        let mut reader = ByteReader::without_metadata(&packet);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("add skill packet was not handled");
        };

        match events.0.as_slice() {
            [NetworkEvent::AddSkill { skill_information }] => {
                assert_eq!(skill_information.skill_id.0, 5);
                assert_eq!(skill_information.skill_level.0, 10);
                assert_eq!(skill_information.spell_point_cost, 8);
                assert_eq!(skill_information.attack_range.0, 1);
                assert_eq!(skill_information.skill_name, "Crazy Uproar");
                assert_eq!(skill_information.upgradable, 1);
            }
            events => panic!("unexpected events: {events:?}"),
        }
    }

    #[test]
    fn skill_info_update_packet_parses_the_2019_layout() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let mut packet = Vec::new();
        packet.extend_from_slice(&0x07E1u16.to_le_bytes());
        packet.extend_from_slice(&5u16.to_le_bytes()); // skill id
        packet.extend_from_slice(&4i32.to_le_bytes()); // type flags (self cast)
        packet.extend_from_slice(&10u16.to_le_bytes()); // level
        packet.extend_from_slice(&8u16.to_le_bytes()); // sp cost
        packet.extend_from_slice(&1u16.to_le_bytes()); // range
        packet.push(0); // upgradable

        assert_eq!(packet.len(), 15);

        let mut reader = ByteReader::without_metadata(&packet);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("skill info update packet was not handled");
        };

        match events.0.as_slice() {
            [
                NetworkEvent::UpdateSkillInfo {
                    skill_id,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                    ..
                },
            ] => {
                assert_eq!(skill_id.0, 5);
                assert_eq!(skill_level.0, 10);
                assert_eq!(*spell_point_cost, 8);
                assert_eq!(attack_range.0, 1);
                assert!(!*upgradable);
            }
            events => panic!("unexpected events: {events:?}"),
        }
    }

    #[test]
    fn skill_tree_packet_parses_multiple_2019_skill_entries() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let first = skill_data_2019(5, 1, 10, 8, 1, "Bash", 1);
        let second = skill_data_2019(155, 4, 5, 30, 1, "Crazy Uproar", 0);

        let mut packet = Vec::new();
        packet.extend_from_slice(&0x010Fu16.to_le_bytes());
        packet.extend_from_slice(&((4 + first.len() + second.len()) as u16).to_le_bytes());
        packet.extend_from_slice(&first);
        packet.extend_from_slice(&second);

        let mut reader = ByteReader::without_metadata(&packet);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("skill tree packet was not handled");
        };

        match events.0.as_slice() {
            [NetworkEvent::SkillTree { skill_information }] => {
                assert_eq!(skill_information.len(), 2);
                assert_eq!(skill_information[0].skill_name, "Bash");
                assert_eq!(skill_information[1].skill_name, "Crazy Uproar");
                assert_eq!(skill_information[1].skill_level.0, 5);
            }
            events => panic!("unexpected events: {events:?}"),
        }
    }

    #[test]
    fn skill_tree_packet_parses_a_full_login_skill_list() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // A full login list for a character with many learned skills: the server sends
        // 4 + 37 * entries bytes (37 byte SKILLDATA per skill) in one 0x010F packet.
        const ENTRIES: u16 = 37;
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x010Fu16.to_le_bytes());
        packet.extend_from_slice(&(4u16 + ENTRIES * 37).to_le_bytes());

        for index in 0..ENTRIES {
            packet.extend_from_slice(&skill_data_2019(
                10 + index,
                1,
                (index % 10) as u16 + 1,
                index,
                1,
                "Test Skill",
                1,
            ));
        }

        assert_eq!(packet.len(), 4 + 37 * 37);

        let mut reader = ByteReader::without_metadata(&packet);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("full skill tree packet was not handled");
        };

        match events.0.as_slice() {
            [NetworkEvent::SkillTree { skill_information }] => {
                assert_eq!(skill_information.len(), 37);
                assert_eq!(skill_information[0].skill_id.0, 10);
                assert_eq!(skill_information[36].skill_id.0, 46);
                assert_eq!(skill_information[10].skill_level.0, 1);
            }
            events => panic!("unexpected events: {events:?}"),
        }
    }

    #[test]
    fn skill_tree_packet_only_consumes_its_declared_entries() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // A skill list followed by another packet in the same TCP read: the variable
        // length parser has to stop exactly at the declared length so that the next
        // packet (a stat update here) is not swallowed or misaligned.
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x010Fu16.to_le_bytes());
        packet.extend_from_slice(&(4u16 + 2 * 37).to_le_bytes());
        packet.extend_from_slice(&skill_data_2019(5, 1, 10, 8, 1, "Bash", 1));
        packet.extend_from_slice(&skill_data_2019(6, 2, 5, 12, 9, "Heal", 1));
        packet.extend_from_slice(&0x00B0u16.to_le_bytes());
        packet.extend_from_slice(&5u16.to_le_bytes()); // stat type: HP
        packet.extend_from_slice(&123u32.to_le_bytes()); // value

        let mut reader = ByteReader::without_metadata(&packet);

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("skill tree packet was not handled");
        };

        match events.0.as_slice() {
            [NetworkEvent::SkillTree { skill_information }] => assert_eq!(skill_information.len(), 2),
            events => panic!("unexpected events: {events:?}"),
        }

        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("following stat update packet was not handled");
        };

        match events.0.as_slice() {
            [NetworkEvent::UpdateStat { stat_type }] => {
                assert_eq!(*stat_type, ragnarok_packets::StatType::HealthPoints(123));
            }
            events => panic!("unexpected events: {events:?}"),
        }
    }

    #[test]
    fn unknown_2019_packet_is_skipped_without_losing_the_skill_list_after_it() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // 0x0B77 is not a registered 2019 packet for this client version. It carries a
        // plausible length followed by a full skill list in the same read: the skip
        // logic has to land exactly on the next (registered) packet header, otherwise
        // the login 0x010F skill list is lost (the original bug).
        let mut data = Vec::new();
        data.extend_from_slice(&0x0FFFu16.to_le_bytes());
        data.extend_from_slice(&12u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]); // payload

        let skill_list_start = data.len();
        data.extend_from_slice(&0x010Fu16.to_le_bytes());
        data.extend_from_slice(&(4u16 + 37).to_le_bytes());
        data.extend_from_slice(&skill_data_2019(5, 1, 10, 8, 1, "Bash", 1));

        let skipped_to = match unknown_packet_action(&data, 0, &handler) {
            UnknownPacketAction::Skip(length) => length,
            action => panic!("expected the unknown packet to be skipped, got {action:?}"),
        };
        assert_eq!(skipped_to, skill_list_start);

        // The skill list itself is then parsed normally.
        let mut reader = ByteReader::without_metadata(&data[skipped_to..]);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("skill list after the skipped packet was not handled");
        };

        match events.0.as_slice() {
            [NetworkEvent::SkillTree { skill_information }] => assert_eq!(skill_information.len(), 1),
            events => panic!("unexpected events: {events:?}"),
        }
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

    /// The unknown packet skip only accepts a declared length when the offset
    /// it points at starts another registered packet (or the end of the
    /// read).
    #[test]
    fn unknown_variable_length_packet_can_be_skipped() {
        let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // 0x0FFF is not a registered packet, followed by a length of 12 bytes and
        // eight bytes of payload; directly after it a known stat packet starts.
        let mut data = vec![0xFF, 0x0F, 12, 0, 1, 2, 3, 4, 5, 6, 7, 8];
        data.extend_from_slice(&[0xB0, 0x00, 20, 0, 100, 0, 0, 0]);

        assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Skip(12));
    }

    #[test]
    fn unknown_packet_ending_the_read_can_be_skipped() {
        let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let data = vec![0xFF, 0x0F, 8, 0, 1, 2, 3, 4];

        assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Skip(8));
    }

    /// Fixed-size packets have no length field, so bytes 2..4 are payload. The
    /// skip must not be used unless the resulting offset starts a known packet.
    #[test]
    fn unknown_fixed_size_packet_is_not_skipped() {
        let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // Bytes 2..4 form a plausible length (20), but the offset it points at is
        // not another packet header and the read is longer, so it must not skip.
        let mut data = vec![0xFF, 0x0F, 20, 0];
        data.extend_from_slice(&[0; 24]);

        assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Discard);

        // A length below the minimum (4) is never accepted.
        let data = vec![0xFF, 0x0F, 3, 0, 1, 2, 3, 4, 5, 6, 7, 8];

        assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Discard);
    }

    /// rAthena sends these party packets during the map login burst of a
    /// character that is in a party (in this order before the NPC/skill burst).
    ///
    /// Every one of them has to be registered with its exact 2019 layout: an
    /// unknown fixed-size packet (0x07D8 has no length field of its own) makes
    /// the stream resync read its payload as a length, fail, and discard the
    /// rest of the TCP read - which swallowed the NPC spawn burst and the whole
    /// skill list for party characters (empty skill tree / no NPCs).
    #[test]
    fn party_login_burst_is_parsed_without_desync() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // 0x07D8 ZC_PARTY_OPTION: exp option (u32), item pickup (u8), item share (u8).
        let mut data = vec![0xD8, 0x07];
        data.extend_from_slice(&1u32.to_le_bytes());
        data.push(1);
        data.push(0);

        // 0x0AE4 ZC_ADD_MEMBER_TO_GROUP (89 bytes).
        data.extend_from_slice(&0x0AE4u16.to_le_bytes());
        data.extend_from_slice(&150_009u32.to_le_bytes()); // account id
        data.extend_from_slice(&150_009u32.to_le_bytes()); // char id
        data.extend_from_slice(&0u32.to_le_bytes()); // role: 0 = leader
        data.extend_from_slice(&7u16.to_le_bytes()); // job
        data.extend_from_slice(&99u16.to_le_bytes()); // base level
        data.extend_from_slice(&155u16.to_le_bytes()); // x
        data.extend_from_slice(&180u16.to_le_bytes()); // y
        data.push(0); // online
        data.extend_from_slice(&fixed_name("feeders", 24));
        data.extend_from_slice(&fixed_name("MrChamp", 24));
        data.extend_from_slice(&fixed_name("prontera", 16));
        data.push(1); // share pickup
        data.push(1); // share loot

        // 0x0AE5 ZC_GROUP_LIST with one member (54 bytes).
        data.extend_from_slice(&0x0AE5u16.to_le_bytes());
        let party_list_length = 4u16 + 24 + 54;
        data.extend_from_slice(&party_list_length.to_le_bytes());
        data.extend_from_slice(&fixed_name("feeders", 24));
        data.extend_from_slice(&150_009u32.to_le_bytes()); // account id
        data.extend_from_slice(&150_009u32.to_le_bytes()); // char id
        data.extend_from_slice(&fixed_name("MrChamp", 24));
        data.extend_from_slice(&fixed_name("prontera", 16));
        data.push(1); // role: 1 = normal member
        data.push(0); // online
        data.extend_from_slice(&7u16.to_le_bytes()); // job
        data.extend_from_slice(&99u16.to_le_bytes()); // base level

        // 0x080E ZC_NOTIFY_HP_TO_GROUPM (14 bytes).
        data.extend_from_slice(&0x080Eu16.to_le_bytes());
        data.extend_from_slice(&150_009u32.to_le_bytes());
        data.extend_from_slice(&7500i32.to_le_bytes());
        data.extend_from_slice(&7500i32.to_le_bytes());

        // The skill list that used to be swallowed.
        data.extend_from_slice(&0x010Fu16.to_le_bytes());
        data.extend_from_slice(&(4u16 + 37).to_le_bytes());
        data.extend_from_slice(&skill_data_2019(5, 1, 10, 8, 1, "Bash", 1));

        let mut reader = ByteReader::without_metadata(&data);
        let mut events = Vec::new();

        while !reader.is_empty() {
            match handler.process_one(&mut reader) {
                HandlerResult::Ok(output) => events.extend(output.0),
                HandlerResult::UnhandledPacket => {
                    let offset = reader.get_offset();
                    let header = u16::from_le_bytes([data[offset], data[offset + 1]]);
                    panic!("party login burst desynchronized at 0x{header:04X}");
                }
                HandlerResult::PacketCutOff => panic!("party login burst was cut off"),
                HandlerResult::InternalError(error) => panic!("party login burst handler failed: {error:?}"),
            }
        }

        assert!(matches!(events[0], NetworkEvent::PartyOption { experience_option: 1, .. }));
        assert!(matches!(&events[1], NetworkEvent::PartyMemberAdded { member } if member.name == "MrChamp" && member.leader));
        assert!(matches!(&events[2], NetworkEvent::PartyList { party_name, members } if party_name == "feeders" && members.len() == 1));
        assert!(matches!(events[3], NetworkEvent::PartyMemberHealth {
            health_points: 7500,
            maximum_health_points: 7500,
            ..
        }));
        assert!(matches!(&events[4], NetworkEvent::SkillTree { skill_information } if skill_information.len() == 1));
    }

    /// ZC_MSG (0x0291) used to be a noop, so msgstringtable system messages
    /// were dropped completely.
    #[test]
    fn message_table_packet_reaches_the_client() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let events = feed(&mut handler, &[0x91, 0x02, 77, 0]);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::MessageString { message_id: 77 }]));
    }

    /// The 0x0ACC experience packet has to reach the client (it was a noop, so
    /// no exp feedback ever appeared). Layout: account id u32, amount i64,
    /// var id u16 (1 = base, 2 = job), exp type u16 (0 = normal, 1 = quest).
    #[test]
    fn experience_gain_packet_reaches_the_client() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let mut data = vec![0xCC, 0x0A];
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&1234i64.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());

        let events = feed(&mut handler, &data);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::GainedExperience {
            amount: 1234,
            experience_type: ragnarok_packets::ExperienceType::BaseExperience,
            experience_source: ragnarok_packets::ExperienceSource::Regular,
        }]));
    }

    /// ZC_ACK_TOUSESKILL (0x0110) used to be a noop, so a failed cast was
    /// completely silent.
    #[test]
    fn skill_use_failure_packet_reaches_the_client() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let mut data = vec![0x10, 0x01];
        data.extend_from_slice(&5u16.to_le_bytes()); // skill id
        data.extend_from_slice(&0u32.to_le_bytes()); // btype
        data.extend_from_slice(&0u32.to_le_bytes()); // item id
        data.push(0); // flag
        data.push(1); // cause: not enough SP

        let events = feed(&mut handler, &data);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::SkillUseFailed {
            skill_id: ragnarok_packets::SkillId(5),
            cause: 1,
        }]));
    }

    /// SC_NOTIFY_BAN (0x0081) on the map server: a 3-byte packet that used to
    /// desynchronize the stream and silently kick the client.
    #[test]
    fn map_server_ban_packet_reaches_the_client() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let events = feed(&mut handler, &[0x81, 0x00, 2]);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::MapServerBan { error_code: 2 }]));
    }

    /// The remaining cheap feedback packets that used to be noops.
    #[test]
    fn small_feedback_packets_reach_the_client() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // ZC_USER_COUNT (0x00C2): header + players count.
        let mut data = vec![0xC2, 0x00];
        data.extend_from_slice(&17i32.to_le_bytes());
        let events = feed(&mut handler, &data);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::UserCount { player_count: 17 }]));

        // ZC_STATUS_CHANGE_ACK (0x00BC): sp u16, ok u8, value u8 (ok = 0 = failure).
        let data = [0xBC, 0x00, 1, 0, 0, 0];
        let events = feed(&mut handler, &data);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::StatUpFailed]));

        // ZC_ACTION_FAILURE (0x013B): uint16 type.
        let data = [0x3B, 0x01, 2, 0];
        let events = feed(&mut handler, &data);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::AmmunitionAction {
            action: ragnarok_packets::AmmunitionActionType::WeightLimitExceeded2,
        }]));

        // ZC_EQUIP_ARROW (0x013C): uint16 inventory index (the wire value is the
        // client index + 2, like every other inventory index field).
        let data = [0x3C, 0x01, 5, 0];
        let events = feed(&mut handler, &data);
        assert!(matches!(events.0.as_slice(), [NetworkEvent::AmmunitionEquipped {
            inventory_index: ragnarok_packets::InventoryIndex(3)
        }]));
    }

    /// Character deletion has to send the real account e-mail address: rAthena
    /// runs with `char_del_option: 1` and compares the 40-byte field with the
    /// account e-mail, so the old hardcoded "a@a.com" made every request fail.
    #[test]
    fn delete_character_request_carries_the_account_email() {
        use ragnarok_packets::*;

        // The packet itself must use the real address, zero padded to 40 bytes
        // and followed by the 10 ignored bytes (56 bytes total for 0x01FB).
        let mut writer = ByteWriter::new();
        DeleteCharacterPacket::new(CharacterId(150_044), "v1@test.local".to_owned())
            .packet_to_bytes(&mut writer)
            .unwrap();
        let bytes = writer.into_inner();

        assert_eq!(&bytes[..2], &[0xFB, 0x01]);
        assert_eq!(&bytes[2..6], &150_044u32.to_le_bytes());
        assert_eq!(&bytes[6..19], b"v1@test.local");
        assert!(bytes[19..46].iter().all(|byte| *byte == 0));
        assert_eq!(bytes.len(), 2 + 4 + 40 + 10);
    }

    /// The party requests have to use the 2019 opcodes and layouts, otherwise
    /// rAthena ignores or misparses them (party play was impossible before).
    #[test]
    fn party_requests_use_the_2019_opcodes_and_layouts() {
        use ragnarok_packets::*;

        let mut writer = ByteWriter::new();
        CreatePartyPacket {
            party_name: "feeders".to_owned(),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        assert_eq!(&writer.as_slice()[..2], &[0xF9, 0x00]);
        assert_eq!(writer.as_slice().len(), 2 + 24);
        assert_eq!(&writer.as_slice()[2..9], b"feeders");

        let mut writer = ByteWriter::new();
        PartyInviteRequestPacket2019 {
            character_name: "MrChamp".to_owned(),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        assert_eq!(&writer.as_slice()[..2], &[0xC4, 0x02]);
        assert_eq!(writer.as_slice().len(), 2 + 24);

        let mut writer = ByteWriter::new();
        PartyInviteReplyRequestPacket2019 {
            party_id: PartyId(13),
            accept: 1,
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        assert_eq!(writer.as_slice(), &[0xC7, 0x02, 13, 0, 0, 0, 1]);

        let mut writer = ByteWriter::new();
        LeavePartyPacket {}.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice(), &[0x00, 0x01]);

        let mut writer = ByteWriter::new();
        ExpelPartyMemberPacket {
            account_id: AccountId(150_009),
            character_name: "MrChamp".to_owned(),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        assert_eq!(&writer.as_slice()[..2], &[0x03, 0x01]);
        assert_eq!(writer.as_slice().len(), 2 + 4 + 24);

        let mut writer = ByteWriter::new();
        PartyChatRequestPacket {
            message: "MrChamp : hello".to_owned(),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        let bytes = writer.into_inner();
        assert_eq!(&bytes[..2], &[0x08, 0x01]);
        let declared_length = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
        assert_eq!(declared_length, bytes.len());
        assert_eq!(&bytes[4..], b"MrChamp : hello\0");
    }

    /// Builds a NUL padded fixed length string field.
    fn fixed_name(name: &str, length: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; length];
        let name_bytes = name.as_bytes();
        let copied = name_bytes.len().min(length - 1);
        bytes[..copied].copy_from_slice(&name_bytes[..copied]);
        bytes
    }

    #[test]
    fn party_login_packets_are_registered() {
        let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        for header in [
            0x07D8u16, 0x0AE4, 0x0AE5, 0x080E, 0x02C5, 0x00FA, 0x0105, 0x0107, 0x0109, 0x0ABD, 0x0AB2,
        ] {
            assert!(
                handler.is_registered(ragnarok_packets::PacketHeader(header)),
                "0x{header:04X} must be registered"
            );
        }
    }

    #[test]
    fn known_headers_are_registered() {
        let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x009C)));
        assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x0B1B)));
        assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x0984)));
        assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x0A37)));
        assert!(!handler.is_registered(ragnarok_packets::PacketHeader(0x0FFF)));
    }

    fn feed(handler: &mut ragnarok_packets::handler::PacketHandler<NetworkEventList, NoPacketCallback>, bytes: &[u8]) -> NetworkEventList {
        let mut reader = ByteReader::without_metadata(bytes);
        let result = handler.process_one(&mut reader);

        match result {
            HandlerResult::Ok(events) => events,
            HandlerResult::UnhandledPacket => panic!("packet 0x{:04X} is not registered", u16::from_le_bytes([bytes[0], bytes[1]])),
            HandlerResult::PacketCutOff => panic!("packet was reported as cut off: {bytes:?}"),
            HandlerResult::InternalError(error) => panic!("packet handler failed: {error:?}"),
        }
    }

    fn regular_item_entry(raw_index: u16) -> Vec<u8> {
        let mut entry = Vec::new();
        entry.extend_from_slice(&raw_index.to_le_bytes()); // index
        entry.extend_from_slice(&501u32.to_le_bytes()); // item id
        entry.push(0); // item type
        entry.extend_from_slice(&5u16.to_le_bytes()); // amount
        entry.extend_from_slice(&0u32.to_le_bytes()); // equipped position
        entry.extend_from_slice(&[0u8; 16]); // cards
        entry.extend_from_slice(&0u32.to_le_bytes()); // hire expiration
        entry.push(0); // flags
        entry
    }

    fn list_packet(header: u16, inventory_type: u8, payload: &[u8]) -> Vec<u8> {
        let length = (4 + 1 + payload.len()) as u16;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&header.to_le_bytes());
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.push(inventory_type);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn list_start(inventory_type: u8, name: &str) -> Vec<u8> {
        let length = (4 + 1 + name.len() + 1) as u16;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0B08u16.to_le_bytes());
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.push(inventory_type);
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
        bytes
    }

    fn list_end(inventory_type: u8) -> Vec<u8> {
        vec![0x0B, 0x0B, inventory_type, 0]
    }

    #[test]
    fn inventory_and_storage_lists_are_routed_by_inventory_type() {
        // Inventory list (type 0): the raw wire index is already `server slot + 2`,
        // so InventoryIndex reports slot 0.
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        feed(&mut handler, &list_start(0, ""));
        feed(&mut handler, &list_packet(0x0B09, 0, &regular_item_entry(2)));
        let events = feed(&mut handler, &list_end(0));

        assert!(
            matches!(events.0.as_slice(), [NetworkEvent::SetInventory { items }] if items.len() == 1 && items[0].index == ragnarok_packets::InventoryIndex(0))
        );

        // Storage list (type 2): the wire index is `server slot + 1`; a storage
        // slot zero is encoded as raw index 1 and must come out as slot 0.
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        feed(&mut handler, &list_start(2, "Storage"));
        feed(&mut handler, &list_packet(0x0B09, 2, &regular_item_entry(1)));
        let events = feed(&mut handler, &list_end(2));

        assert!(
            matches!(events.0.as_slice(), [NetworkEvent::SetStorage { items, name }] if items.len() == 1 && items[0].index == ragnarok_packets::InventoryIndex(0) && name == "Storage")
        );
    }

    #[test]
    fn cart_and_guild_storage_lists_are_ignored_without_desync() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        feed(&mut handler, &list_start(1, ""));
        feed(&mut handler, &list_packet(0x0B09, 1, &regular_item_entry(2)));
        let events = feed(&mut handler, &list_end(1));

        assert!(events.0.is_empty());
    }

    #[test]
    fn storage_add_remove_amount_and_close_packets() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // 0x0A0A ZC_ADD_ITEM_TO_STORE, 2019 layout (57 bytes).
        let mut added = Vec::new();
        added.extend_from_slice(&0x0A0Au16.to_le_bytes());
        added.extend_from_slice(&1u16.to_le_bytes()); // storage slot 0 (+1)
        added.extend_from_slice(&5i32.to_le_bytes()); // amount
        added.extend_from_slice(&501u32.to_le_bytes()); // item id
        added.extend_from_slice(&[0, 1, 0, 0]); // type, identified, damaged, refine
        added.extend_from_slice(&[0u8; 16]); // cards
        added.extend_from_slice(&[0u8; 25]); // options
        assert_eq!(added.len(), 57);

        let events = feed(&mut handler, &added);
        assert!(
            matches!(events.0.as_slice(), [NetworkEvent::StorageItemAdded { item }] if item.index == ragnarok_packets::InventoryIndex(0))
        );

        // 0x00F2 amount, 0x00F6 remove, 0x00F8 close.
        let mut amount = Vec::new();
        amount.extend_from_slice(&0x00F2u16.to_le_bytes());
        amount.extend_from_slice(&3u16.to_le_bytes());
        amount.extend_from_slice(&300u16.to_le_bytes());
        assert!(matches!(feed(&mut handler, &amount).0.as_slice(), [
            NetworkEvent::SetStorageAmount {
                amount: 3,
                maximum_amount: 300
            }
        ]));

        let mut removed = Vec::new();
        removed.extend_from_slice(&0x00F6u16.to_le_bytes());
        removed.extend_from_slice(&1u16.to_le_bytes());
        removed.extend_from_slice(&2u32.to_le_bytes());
        assert!(matches!(feed(&mut handler, &removed).0.as_slice(), [
            NetworkEvent::StorageItemRemoved {
                storage_index: 0,
                amount: 2
            }
        ]));

        assert!(matches!(feed(&mut handler, &[0xF8, 0x00]).0.as_slice(), [
            NetworkEvent::StorageClosed
        ]));
    }

    #[test]
    fn storage_deposit_and_withdraw_packets_use_the_expected_index_encoding() {
        use ragnarok_packets::{CloseStoragePacket, PacketExt, StorageDepositPacket, StorageWithdrawPacket};

        let mut writer = ByteWriter::new();
        StorageDepositPacket {
            inventory_index: ragnarok_packets::InventoryIndex(0),
            amount: 5,
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        let bytes = writer.into_inner();
        assert_eq!(bytes.len(), 8);
        // 2019 uses 0x07EC for the deposit request.
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x07EC);
        // Inventory index 0 is encoded as 2.
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 2);

        let mut writer = ByteWriter::new();
        StorageWithdrawPacket {
            storage_index: 1,
            amount: 5,
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        let bytes = writer.into_inner();
        assert_eq!(bytes.len(), 8);
        // 2019 uses 0x0861 for the withdraw request.
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0861);
        // The caller passes the storage slot + 1.
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 1);

        let mut writer = ByteWriter::new();
        CloseStoragePacket {}.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.into_inner(), vec![0x93, 0x01]);
    }

    #[test]
    fn item_add_packet_updates_the_inventory() {
        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // 0x0A37 ZC_ITEM_PICKUP_ACK, 69 byte 2019 layout (EQUIPSLOTINFO is u32[4]);
        // index is client_index (slot 2 -> raw 4), a successful add of 7 Red
        // Potions to slot 2.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0A37u16.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes()); // index
        bytes.extend_from_slice(&7u16.to_le_bytes()); // amount
        bytes.extend_from_slice(&501u32.to_le_bytes()); // item id
        bytes.extend_from_slice(&[1, 0, 0]); // identified, damaged, refine
        bytes.extend_from_slice(&[0u8; 16]); // cards (u32[4])
        bytes.extend_from_slice(&0u32.to_le_bytes()); // equip position
        bytes.extend_from_slice(&[0, 0]); // item type, result
        bytes.extend_from_slice(&0i32.to_le_bytes()); // hire expiration
        bytes.extend_from_slice(&0u16.to_le_bytes()); // bind on equip type
        bytes.extend_from_slice(&[0u8; 25]); // options
        bytes.extend_from_slice(&[0u8, 0, 0]); // favorite + item sprite number
        assert_eq!(bytes.len(), 69);

        let events = feed(&mut handler, &bytes);
        // The 2019 pickup ack adds the item to the inventory; the "you got"
        // chat message is produced by the client from the same event elsewhere.
        assert!(matches!(events.0.as_slice(), [
            NetworkEvent::IventoryItemAdded { item },
        ] if item.index == ragnarok_packets::InventoryIndex(2)));
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
