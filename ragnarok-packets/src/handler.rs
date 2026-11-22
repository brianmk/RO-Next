use std::collections::HashMap;

use ragnarok_bytes::{ByteReader, ConversionError, ConversionResult, FromBytes};

use crate::PacketHeader;

/// Possible results of [`PacketHandler::process_one`].
pub enum HandlerResult<Output> {
    /// Packet was successfully processed and produced some output.
    Ok(Output),
    /// No packet handler was registered for the incoming packet.
    UnhandledPacket,
    /// Packet was most likely cut-off.
    PacketCutOff,
    /// An error occurred inside the packet handler.
    InternalError(Box<ConversionError>),
}

/// Error when trying to register two separate handlers for the same packet.
#[derive(Debug, Clone)]
pub struct DuplicateHandlerError {
    /// Header of the packet.
    pub packet_header: PacketHeader,
}

/// Trait for monitoring the incoming and outgoing packets.
pub trait PacketCallback: Clone + 'static {
    /// Called by the [`PacketHandler`] when a packet is received.
    fn incoming_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let _ = packet;
    }

    /// Called by when a packet is sent.
    fn outgoing_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let _ = packet;
    }

    /// Called by the [`PacketHandler`] when a packet arrives that doesn't have
    /// a handler registered.
    fn unknown_packet(&self, bytes: Vec<u8>) {
        let _ = bytes;
    }

    /// Called by the [`PacketHandler`] when a packet handler returned an error.
    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ConversionError>) {
        let _ = (bytes, error);
    }
}

#[derive(Debug, Default, Clone)]
pub struct NoPacketCallback;

impl PacketCallback for NoPacketCallback {}

pub type HandlerFunction<Output> = Box<dyn Fn(&mut ByteReader) -> ConversionResult<Output>>;

/// Layout metadata of a registered handler, taken from the packet trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketMetadata {
    /// Whether the packet declares a variable length.
    pub is_variable_length: bool,
    /// The fixed wire size declared by `#[packet_size(N)]`, if any.
    pub expected_size: Option<usize>,
}

/// A struct to help with reading packets from a [`ByteReader`] and
/// converting them to some common event type.
///
/// It allows passing a packet callback to monitor incoming packets.
pub struct PacketHandler<Output, Callback> {
    handlers: HashMap<PacketHeader, HandlerFunction<Output>>,
    metadata: HashMap<PacketHeader, PacketMetadata>,
    packet_callback: Callback,
}

impl<Output, Callback> Default for PacketHandler<Output, Callback>
where
    Callback: Default,
{
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            metadata: Default::default(),
            packet_callback: Default::default(),
        }
    }
}

impl<Output, Callback> PacketHandler<Output, Callback>
where
    Output: Default,
    Callback: PacketCallback,
{
    /// Create a new packet handler with a callback.
    pub fn with_callback(packet_callback: Callback) -> Self {
        Self {
            handlers: Default::default(),
            metadata: Default::default(),
            packet_callback,
        }
    }

    /// Register a new packet handler.
    pub fn register<Packet, Return>(&mut self, handler: impl Fn(Packet) -> Return + 'static) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
        Return: Into<Output>,
    {
        let packet_callback = self.packet_callback.clone();
        self.metadata.insert(Packet::HEADER, PacketMetadata {
            is_variable_length: Packet::IS_VARIABLE_LENGTH,
            expected_size: Packet::EXPECTED_SIZE,
        });
        let old_handler = self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                packet_callback.incoming_packet(&packet);

                Ok(handler(packet).into())
            }),
        );

        match old_handler.is_some() {
            true => Err(DuplicateHandlerError {
                packet_header: Packet::HEADER,
            }),
            false => Ok(()),
        }
    }

    /// Register a noop packet handler.
    pub fn register_noop<Packet>(&mut self) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
    {
        let packet_callback = self.packet_callback.clone();
        self.metadata.insert(Packet::HEADER, PacketMetadata {
            is_variable_length: Packet::IS_VARIABLE_LENGTH,
            expected_size: Packet::EXPECTED_SIZE,
        });
        let old_handler = self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                packet_callback.incoming_packet(&packet);

                Ok(Output::default())
            }),
        );

        match old_handler.is_some() {
            true => Err(DuplicateHandlerError {
                packet_header: Packet::HEADER,
            }),
            false => Ok(()),
        }
    }

    /// Returns whether a handler is registered for the given packet header.
    ///
    /// This is used by the networking loop to resynchronize the stream after an
    /// unknown packet by validating that the bytes following the packet's
    /// declared length actually start another known packet.
    pub fn is_registered(&self, packet_header: PacketHeader) -> bool {
        self.handlers.contains_key(&packet_header)
    }

    /// Returns the layout metadata of a registered packet header.
    pub fn metadata(&self, packet_header: PacketHeader) -> Option<PacketMetadata> {
        self.metadata.get(&packet_header).copied()
    }

    /// Returns all registered packet headers, sorted ascending.
    ///
    /// Used by the contract tests to iterate every registered server packet and
    /// by tooling that needs to know the complete registration set.
    pub fn registered_headers(&self) -> Vec<PacketHeader> {
        let mut headers: Vec<PacketHeader> = self.handlers.keys().copied().collect();
        headers.sort();
        headers
    }

    /// Take a single packet from the byte stream.
    pub fn process_one(&mut self, byte_reader: &mut ByteReader) -> HandlerResult<Output> {
        let save_point = byte_reader.create_save_point();

        let Ok(header) = PacketHeader::from_bytes(byte_reader) else {
            // Packet is cut-off at the header.
            byte_reader.restore_save_point(save_point);
            return HandlerResult::PacketCutOff;
        };

        let Some(handler) = self.handlers.get(&header) else {
            byte_reader.restore_save_point(save_point);

            self.packet_callback.unknown_packet(byte_reader.remaining_bytes());

            return HandlerResult::UnhandledPacket;
        };

        match handler(byte_reader) {
            Ok(output) => HandlerResult::Ok(output),
            // Cut-off packet (probably).
            Err(error) if error.is_byte_reader_too_short() => {
                byte_reader.restore_save_point(save_point);
                HandlerResult::PacketCutOff
            }
            Err(error) => {
                byte_reader.restore_save_point(save_point);

                self.packet_callback.failed_packet(byte_reader.remaining_bytes(), error.clone());

                HandlerResult::InternalError(error)
            }
        }
    }
}
