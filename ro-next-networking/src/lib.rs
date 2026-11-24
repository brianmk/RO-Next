#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]
#![cfg_attr(feature = "interface", feature(negative_impls))]

mod entity;
mod event;
mod hotkey;
pub mod item_type;
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
pub use self::items::{InventoryItem, InventoryItemDetails, ItemQuantity, NoMetadata, SellItem, ShopItem, VendingShopItem};
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

/// Report a failure to build one of the server packet handlers.
///
/// Duplicate handler registration is a client build bug, not a server problem.
/// It must not panic inside the networking thread: report the exact packet
/// header in the log and tell the client to show the error window, then skip
/// the connection attempt.
fn report_packet_handler_registration_failure(
    server: &'static str,
    error: &DuplicateHandlerError,
    event_sender: &UnboundedSender<NetworkEvent>,
) {
    eprintln!("[network] failed to create the {server} server packet handler: {error}");
    let _ = event_sender.send(NetworkEvent::InternalError {
        message: format!("Client setup failed for the {server} server: {error}. Please report this bug."),
    });
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
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
                // The networking thread cannot run without a runtime. Failing
                // gracefully returns from the thread (the command channel is
                // dropped), which the client reports as a disconnected server
                // instead of aborting the process.
                eprintln!("[network] failed to create the Tokio runtime; networking is unavailable");
                return;
            };

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

                            let packet_handler = match Self::create_login_server_packet_handler(packet_callback.clone(), packet_version) {
                                Ok(packet_handler) => packet_handler,
                                Err(error) => {
                                    report_packet_handler_registration_failure("login", &error, &event_sender);
                                    continue;
                                }
                            };
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

                            let packet_handler = match Self::create_character_server_packet_handler(packet_callback.clone(), packet_version)
                            {
                                Ok(packet_handler) => packet_handler,
                                Err(error) => {
                                    report_packet_handler_registration_failure("character", &error, &event_sender);
                                    continue;
                                }
                            };
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

                            let packet_handler = match Self::create_map_server_packet_handler(packet_callback.clone(), packet_version) {
                                Ok(packet_handler) => packet_handler,
                                Err(error) => {
                                    report_packet_handler_registration_failure("map", &error, &event_sender);
                                    continue;
                                }
                            };
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
                                let Ok(account_id) = AccountId::from_bytes(&mut byte_reader) else {
                                    // Reading a `u32` cannot fail with four bytes
                                    // available, but never panic in the networking
                                    // thread if that invariant changes.
                                    break Err(NetworkTaskError::ConnectionClosed);
                                };
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

    /// Claims the reward of a completed achievement (`CZ_REQ_ACH_REWARD`,
    /// 0x0A25). The answer arrives as [`NetworkEvent::AchievementRewardResult`]
    /// and, on success, as a `ZC_ACH_UPDATE` with the `rewarded` flag set.
    pub fn request_achievement_reward(&mut self, achievement_id: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(AchievementRewardRequestPacket { achievement_id }),
        }
    }

    /// Requests the emblem image of a guild (`CZ_REQ_GUILD_EMBLEM_IMG`,
    /// `0x0151`).
    pub fn request_guild_emblem(&mut self, guild_id: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildEmblemRequestPacket::new(guild_id)),
        }
    }

    /// Requests the guild window tab/master mask
    /// (`CZ_REQ_GUILD_MENUINTERFACE`, `0x014D`).
    pub fn request_guild_menu_interface(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildMenuInterfaceRequestPacket {}),
        }
    }

    /// Requests one guild window tab (`CZ_REQ_GUILD_MENU`, `0x014F`).
    ///
    /// `menu` is the rAthena `clif_parse_GuildRequestInfo` type: 0 = basic
    /// info, 1 = members, 2 = positions, 3 = skills, 4 = expulsion list.
    pub fn request_guild_info(&mut self, menu: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildMenuRequestPacket { menu }),
        }
    }

    /// Requests a guild member's character name by character id
    /// (`CZ_REQNAME_BYGID`, `0x094C` at PACKETVER 20190605).
    pub fn request_guild_member_name(&mut self, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildMemberNameRequestPacket { character_id }),
        }
    }

    /// Invites a character to the guild by name (`CZ_REQ_JOIN_GUILD2`,
    /// `0x0916`).
    pub fn invite_to_guild(&mut self, name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildInviteRequestPacket { name }),
        }
    }

    /// Leaves the guild (`CZ_REQ_LEAVE_GUILD`, `0x0159`).
    pub fn leave_guild(&mut self, guild_id: u32, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildLeaveRequestPacket {
                guild_id,
                account_id,
                character_id,
                message: String::new(),
            }),
        }
    }

    /// Expels a guild member (`CZ_REQ_BAN_GUILD`, `0x015B`).
    pub fn expel_guild_member(
        &mut self,
        guild_id: u32,
        account_id: AccountId,
        character_id: CharacterId,
        message: String,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildExpelRequestPacket {
                guild_id,
                account_id,
                character_id,
                message,
            }),
        }
    }

    /// Changes the guild notice (`CZ_REQ_GUILD_NOTICE`, `0x016E`).
    pub fn change_guild_notice(&mut self, guild_id: u32, subject: String, notice: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildNoticeChangeRequestPacket { guild_id, subject, notice }),
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

    /// Create a chat room (`CZ_CREATE_CHATROOM`, 0x00D5).
    ///
    /// `room_type` is `0` for a password protected room and `1` for a public
    /// one, like the `type` field of the 2019 packet.
    pub fn create_chat_room(&mut self, title: String, password: String, limit: u16, room_type: u8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CreateChatRoomPacket {
                limit,
                room_type,
                password,
                title,
            }),
        }
    }

    /// Join a chat room (`CZ_REQ_ENTER_ROOM`, 0x00D9).
    pub fn enter_chat_room(&mut self, chat_id: u32, password: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(EnterChatRoomRequestPacket { chat_id, password }),
        }
    }

    /// Leave the current chat room (`CZ_EXIT_ROOM`, 0x00E3).
    pub fn leave_chat_room(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ExitChatRoomPacket {}),
        }
    }

    /// Answer the character server's pincode prompt.
    ///
    /// `pin` is the already obfuscated four byte value (see
    /// `ro_next::pincode::encrypt_pincode`). `is_new` selects
    /// `CZ_PINCODE_SET_NEW` (0x08BA) instead of `CZ_PINCODE_CHECK` (0x08B8).
    pub fn send_pincode(&mut self, account_id: AccountId, pin: [u8; 4], is_new: bool) -> Result<(), NotConnectedError> {
        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => match is_new {
                true => self.send_character_server_packet(PincodeSetNewPacket { account_id, pin }),
                false => self.send_character_server_packet(PincodeCheckPacket { account_id, pin }),
            },
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

    /// Move items from the inventory into the merchant pushcart
    /// (`CZ_MOVE_ITEM_FROM_BODY_TO_CART`, 0x0126, `clif.cpp:12544`).
    pub fn move_item_to_cart(&mut self, inventory_index: InventoryIndex, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MoveItemToCartPacket { inventory_index, amount }),
        }
    }

    /// Move items from the pushcart back into the inventory
    /// (`CZ_MOVE_ITEM_FROM_CART_TO_BODY`, 0x0127, `clif.cpp:12558`).
    pub fn move_item_from_cart(&mut self, cart_index: InventoryIndex, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MoveItemFromCartPacket { cart_index, amount }),
        }
    }

    /// Move items from the open Kafra storage into the pushcart
    /// (`CZ_MOVE_ITEM_FROM_STORE_TO_CART`, 0x0128, `clif.cpp:13654`). The
    /// server expects the storage slot encoded as `slot + 1`.
    pub fn move_storage_item_to_cart(&mut self, storage_index: u16, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MoveItemFromStorageToCartPacket {
                storage_index: storage_index.saturating_add(1),
                amount,
            }),
        }
    }

    /// Move items from the pushcart into the open Kafra storage
    /// (`CZ_MOVE_ITEM_FROM_CART_TO_STORE`, 0x0129, `clif.cpp:13627`).
    pub fn move_cart_item_to_storage(&mut self, cart_index: InventoryIndex, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MoveItemFromCartToStoragePacket { cart_index, amount }),
        }
    }

    /// Ask the map server for another player's vending shop list
    /// (`CZ_REQ_BUY_FROMMC`, 0x0130, `clif.cpp:14128`). The server answers with
    /// `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800).
    pub fn request_vending_list(&mut self, owner_account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestVendingListPacket { owner_account_id }),
        }
    }

    /// Buy `(cart index, amount)` rows from another player's vending shop
    /// (`CZ_PC_PURCHASE_ITEMLIST_FROMMC`, 0x0134, `clif.cpp:14138`). The cart
    /// index is the value the server sent in the shop list.
    pub fn purchase_from_vending(
        &mut self,
        owner_account_id: AccountId,
        items: Vec<(InventoryIndex, u16)>,
    ) -> Result<(), NotConnectedError> {
        let items = items
            .into_iter()
            .map(|(cart_index, amount)| PurchaseFromShopEntry { amount, cart_index })
            .collect();

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PurchaseFromShopPacket { owner_account_id, items }),
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

    /// Answers a Warp Portal destination list (`ZC_WARPLIST`, 0x0ABE) with the
    /// chosen destination map. The server then creates the portal units.
    pub fn select_warp_point(&mut self, skill_id: SkillId, map_name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SelectWarpPointPacket::new(skill_id, map_name)),
        }
    }

    /// Requests a trade with another player (`CZ_REQ_EXCHANGE_ITEM`).
    pub fn request_trade(&mut self, target_account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestExchangeItemPacket { target_account_id }),
        }
    }

    /// Answers a trade request (`CZ_ACK_EXCHANGE_ITEM`). `accept` true sends 3.
    pub fn answer_trade(&mut self, accept: bool) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(AnswerExchangeItemPacket {
                result: if accept { 3 } else { 4 },
            }),
        }
    }

    /// Offers an item (or zeny with index 0) in the current trade.
    pub fn add_trade_item(&mut self, inventory_index: u16, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(AddExchangeItemPacket { inventory_index, amount }),
        }
    }

    /// Locks the local side of the trade (`CZ_CONCLUDE_EXCHANGE_ITEM`).
    pub fn confirm_trade(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ConfirmExchangeItemPacket {}),
        }
    }

    /// Cancels the current trade (`CZ_CANCEL_EXCHANGE_ITEM`).
    pub fn cancel_trade(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CancelExchangeItemPacket {}),
        }
    }

    /// Commits the trade (`CZ_EXEC_EXCHANGE_ITEM`).
    pub fn commit_trade(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CommitExchangeItemPacket {}),
        }
    }

    /// Creates a vending shop (`CZ_REQ_OPENSTORE`, 0x012F). `items` are
    /// (inventory index, amount, price) entries.
    pub fn request_open_store(&mut self, shop_name: String, items: Vec<StoreItemEntry>) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestOpenStorePacket { shop_name, items }),
        }
    }

    /// Cancels the vending shop creation UI (`CZ_REQ_OPENSTORE2`, 0x01B2).
    pub fn cancel_open_store(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CancelOpenStorePacket {
                shop_name: String::new(),
                result: 0,
                items: Vec::new(),
            }),
        }
    }

    /// Use an item from the inventory (potions, usable items, ...).
    pub fn use_item(&mut self, item_index: InventoryIndex, item_id: ItemId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(UseItemPacket::new(item_index, item_id)),
        }
    }

    pub fn input_edit_dialog(&mut self, npc_id: EntityId, value: i32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(InputEditDialogPacket::new(npc_id, value)),
        }
    }

    pub fn input_edit_dialog_str(&mut self, npc_id: EntityId, value: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(InputEditDialogStrPacket::new(npc_id, value)),
        }
    }

    pub fn drop_item(&mut self, item_index: InventoryIndex, amount: u16) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(DropItemPacket::new(item_index, amount)),
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

    /// Select the weapon to refine (`CZ_REQ_WEAPONREFINE`, 0x0222).
    ///
    /// Only accepted by rAthena while the `WS_WEAPONREFINE` menu skill is
    /// pending, i.e. after the server sent `ZC_NOTIFY_WEAPONITEMLIST`
    /// (`clif_parse_WeaponRefine`, `clif.cpp:13255`). The request itself
    /// carries a 32 bit client index, which is why the packet constructor
    /// does the +2 encoding.
    pub fn refine_weapon(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(WeaponRefineRequestPacket::for_inventory_index(item_index)),
        }
    }

    /// Repair a damaged item (`CZ_REQ_ITEMREPAIR1`, 0x01FD).
    ///
    /// The index is the raw 0-based inventory slot, exactly like the values of
    /// `ZC_REPAIRITEMLIST` (rAthena writes/reads them without the +2 client
    /// index encoding, `clif.cpp:7153`/`13242`). rAthena only reads the index
    /// from the request; `item_id`/`refine`/`card` are informational.
    pub fn repair_item(
        &mut self,
        inventory_index: InventoryIndex,
        item_id: ItemId,
        refine: u8,
        card: [u32; 4],
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RepairItemRequestPacket {
                index: inventory_index.0,
                item_id,
                refine,
                card,
            }),
        }
    }

    /// Answer the identify item selection dialog (`CZ_REQ_ITEMIDENTIFY`,
    /// 0x0178). Only accepted while the server has the `MC_IDENTIFY` menu skill
    /// pending (`clif.cpp:13385`).
    pub fn identify_item(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(IdentifyItemRequestPacket { index: item_index }),
        }
    }

    /// Identify an item with one click (`CZ_REQ_ONECLICK_ITEMIDENTIFY`,
    /// 0x0A35), consuming a Magnifier from the inventory. Unlike
    /// [`Self::identify_item`] this needs no pending menu skill
    /// (`clif_parse_Oneclick_Itemidentify`, `clif.cpp:21385`).
    pub fn one_click_identify(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(OneClickIdentifyRequestPacket { index: item_index }),
        }
    }

    /// Request the equipment list a card can be inserted into
    /// (`CZ_REQ_ITEMCOMPOSITION_LIST`, 0x017A).
    pub fn request_card_composition_list(&mut self, card_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CardCompositionListRequestPacket { index: card_index }),
        }
    }

    /// Insert the selected card into the selected equipment
    /// (`CZ_REQ_ITEMCOMPOSITION`, 0x017C).
    pub fn insert_card(&mut self, card_index: InventoryIndex, equip_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CardCompositionRequestPacket { card_index, equip_index }),
        }
    }

    /// Select the item to refine in the refine UI (`CZ_REFINING_SELECT_ITEM`,
    /// 0x0AA1). The server answers with `ZC_REFINING_MATERIAL_LIST` (0x0AA2).
    pub fn select_refining_item(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RefiningSelectItemRequestPacket { index: item_index }),
        }
    }

    /// Attempt a refine in the refine UI (`CZ_REQ_REFINING`, 0x0AA3).
    pub fn request_refining(
        &mut self,
        item_index: InventoryIndex,
        material_item_id: ItemId,
        blacksmith_blessing: bool,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RefiningRequestPacket {
                index: item_index,
                material_item_id,
                blacksmith_blessing: i8::from(blacksmith_blessing),
            }),
        }
    }

    /// Close the refine UI (`CZ_CLOSE_REFINING_UI`, 0x0AA4).
    pub fn close_refining_ui(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CloseRefiningUiRequestPacket {}),
        }
    }

    /// Cancel the identify list (`CZ_REQ_ITEMIDENTIFY`, 0x0178, index `-1`).
    ///
    /// rAthena sets `WIP_DISABLE_ALL` while the identify list is open, which
    /// blocks item and skill use; only the identify request or the official
    /// cancel value clears it (`clif.cpp:13394`-`13400`). The cancel value on
    /// the wire is `0xFFFF` (`InventoryIndex(0xFFFD)` + 2).
    pub fn cancel_identify_item(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(IdentifyItemRequestPacket {
                index: InventoryIndex(u16::MAX - 2),
            }),
        }
    }

    /// Cancel the weapon refine list (`CZ_REQ_WEAPONREFINE`, 0x0222, index
    /// `0xFFFF`). rAthena's `skill_weaponrefine` ignores out-of-range indexes
    /// and then clears the menu skill (`clif.cpp:13262`).
    pub fn cancel_weapon_refine(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(WeaponRefineRequestPacket {
                client_index: u32::from(u16::MAX),
            }),
        }
    }

    /// Cancel the repair item list (`CZ_REQ_ITEMREPAIR1`, 0x01FD, index
    /// `0xFFFF`). `skill_repairweapon` treats `0xFFFF` as "cancel clicked"
    /// (`skill.cpp:20604`) and the parser clears the menu skill afterwards
    /// (`clif.cpp:13243`).
    pub fn cancel_repair_item(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RepairItemRequestPacket {
                index: u16::MAX,
                item_id: ItemId(0),
                refine: 0,
                card: [0; 4],
            }),
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

    /// Register (or refresh) a party-booking advertisement
    /// (`CZ_PARTY_BOOKING_REQ_REGISTER`, 0x0802).
    ///
    /// `jobs` are the six desired job ids; `u16::MAX` means "any job". The
    /// server answers with `ZC_PARTY_BOOKING_ACK_REGISTER` (0x0803).
    pub fn register_party_booking(&mut self, level: u16, map_id: u16, jobs: [u16; 6]) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyBookingRegisterRequestPacket2019 { level, map_id, jobs }),
        }
    }

    /// Search the party-booking board (`CZ_PARTY_BOOKING_REQ_SEARCH`, 0x0804).
    ///
    /// `job` is `-1` for any job, `last_index` pages through the result list
    /// and `result_count` is the maximum number of rows requested.
    pub fn search_party_booking(
        &mut self,
        level: u16,
        map_id: u16,
        job: i16,
        last_index: u32,
        result_count: u16,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyBookingSearchRequestPacket2019 {
                level,
                map_id,
                job,
                last_index,
                result_count,
            }),
        }
    }

    /// Delete the own party-booking advertisement
    /// (`CZ_PARTY_BOOKING_REQ_DELETE`, 0x0806).
    pub fn delete_party_booking(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyBookingDeleteRequestPacket2019 {}),
        }
    }

    /// Change the desired jobs of the own advertisement
    /// (`CZ_PARTY_BOOKING_REQ_UPDATE`, 0x0808).
    pub fn update_party_booking(&mut self, jobs: [u16; 6]) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyBookingUpdateRequestPacket2019 { jobs }),
        }
    }

    /// Ask a party leader to join through the booking board
    /// (`CZ_PARTY_REQ_MASTER_TO_JOIN`, 0x0AE6).
    ///
    /// The server only parses this packet with `PACKETVER >= 20191204`; the
    /// caller has to gate the action (the CondaRO 20190605 server does not
    /// support it).
    pub fn request_party_booking_join(&mut self, character_id: CharacterId, account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyBookingJoinRequestPacket { character_id, account_id }),
        }
    }

    /// Accept or reject a booking join request as the party leader
    /// (`CZ_PARTY_REQ_ACK_MASTER_TO_JOIN`, 0x0AF8).
    pub fn reply_party_booking_join(
        &mut self,
        character_id: CharacterId,
        account_id: AccountId,
        accept: bool,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyBookingJoinReplyPacket {
                character_id,
                account_id,
                accept: accept as u8,
            }),
        }
    }

    /// Change the party EXP sharing and item rules
    /// (`CZ_GROUPINFO_CHANGE_V2`, 0x07D7).
    pub fn change_party_option(&mut self, experience_option: u32, item_pickup: bool, item_share: bool) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyOptionRequestPacket2019 {
                experience_option,
                item_pickup: item_pickup as u8,
                item_share: item_share as u8,
            }),
        }
    }

    /// Transfer party leadership (`CZ_CHANGE_GROUP_MASTER`, 0x07DA).
    pub fn change_party_leader(&mut self, account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ChangePartyLeaderPacket { account_id }),
        }
    }

    /// Open the Rodex mailbox and request the inbox (`CZ_REQ_OPEN_MAILBOX2`,
    /// 0x0AC0). All upper mail ids are zero: the first page of the normal tab.
    pub fn open_mailbox(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MailboxOpenRequestPacket {
                mail_id: 0,
                unknown: [0; 16],
            }),
        }
    }

    /// Force a mailbox refresh (`CZ_REQ_REFRESH_MAIL_LIST2`, 0x0AC1).
    pub fn refresh_mailbox(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MailboxRefreshRequestPacket {
                mail_id: 0,
                unknown: [0; 16],
            }),
        }
    }

    /// Request the body of one mail (`CZ_REQ_READ_MAIL`, 0x09EA).
    pub fn read_mail(&mut self, mail_id: u64, open_type: u8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MailReadRequestPacket { open_type, mail_id }),
        }
    }

    /// Claim the zeny attachment of a mail (`CZ_REQ_ZENY_FROM_MAIL`, 0x09F1).
    pub fn claim_mail_zeny(&mut self, mail_id: u64, open_type: u8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MailClaimZenyRequestPacket { mail_id, open_type }),
        }
    }

    /// Claim the item attachments of a mail (`CZ_REQ_ITEM_FROM_MAIL`, 0x09F3).
    pub fn claim_mail_items(&mut self, mail_id: u64, open_type: u8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MailClaimItemRequestPacket { mail_id, open_type }),
        }
    }

    /// Delete a mail (`CZ_REQ_DELETE_MAIL`, 0x09F5). The server refuses to
    /// delete a mail that still has attachments.
    pub fn delete_mail(&mut self, mail_id: u64, open_type: u8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MailDeleteRequestPacket { open_type, mail_id }),
        }
    }

    /// Send a guild chat message (`CZ_GUILD_CHAT`, 0x017E).
    ///
    /// The server validates the `<name> : <message>` format exactly like public
    /// and party chat (see `clif_process_message` in the rAthena source).
    pub fn send_guild_chat_message(&mut self, player_name: &str, text: &str) -> Result<(), NotConnectedError> {
        let message = format!("{} : {}", player_name, text);

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GuildChatRequestPacket { message }),
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

    /// Open the banking session (`CZ_REQ_OPEN_BANKING`, 0x09B6). rAthena only
    /// answers when the account id matches the sender.
    pub fn open_bank(&mut self, account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BankOpenRequestPacket { account_id }),
        }
    }

    /// Close the banking session (`CZ_REQ_CLOSE_BANKING`, 0x09B8).
    pub fn close_bank(&mut self, account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BankCloseRequestPacket { account_id }),
        }
    }

    /// Request the current bank vault (`CZ_REQ_BANKING_CHECK`, 0x09AB).
    pub fn check_bank(&mut self, account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BankCheckRequestPacket { account_id }),
        }
    }

    /// Move zeny into the bank vault (`CZ_REQ_BANKING_DEPOSIT`, 0x09A7).
    pub fn bank_deposit(&mut self, account_id: AccountId, zeny: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BankDepositRequestPacket { account_id, zeny }),
        }
    }

    /// Move zeny out of the bank vault (`CZ_REQ_BANKING_WITHDRAW`, 0x09A9).
    pub fn bank_withdraw(&mut self, account_id: AccountId, zeny: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BankWithdrawRequestPacket { account_id, zeny }),
        }
    }
}

#[cfg(test)]
mod contract_2019;

#[cfg(test)]
mod tests;
