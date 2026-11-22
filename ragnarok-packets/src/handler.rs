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
///
/// The error always carries the exact [`PacketHeader`] that was already
/// registered, so the connection setup can name the offending packet in its
/// logs and error window instead of panicking inside the networking thread.
#[derive(Debug, Clone)]
pub struct DuplicateHandlerError {
    /// Header of the packet.
    pub packet_header: PacketHeader,
}

impl DuplicateHandlerError {
    /// Create an error for a packet header that is already registered.
    pub fn new(packet_header: PacketHeader) -> Self {
        Self { packet_header }
    }
}

impl std::fmt::Display for DuplicateHandlerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "duplicate packet handler registration for header 0x{:04X}",
            self.packet_header.0
        )
    }
}

impl std::error::Error for DuplicateHandlerError {}

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
    ///
    /// A duplicate registration returns a [`DuplicateHandlerError`] carrying
    /// the exact header. The first registration stays in place: a duplicate
    /// must never silently replace a handler that is already in use.
    pub fn register<Packet, Return>(&mut self, handler: impl Fn(Packet) -> Return + 'static) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
        Return: Into<Output>,
    {
        if self.handlers.contains_key(&Packet::HEADER) {
            return Err(DuplicateHandlerError::new(Packet::HEADER));
        }

        let packet_callback = self.packet_callback.clone();
        self.metadata.insert(Packet::HEADER, PacketMetadata {
            is_variable_length: Packet::IS_VARIABLE_LENGTH,
            expected_size: Packet::EXPECTED_SIZE,
        });
        self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                packet_callback.incoming_packet(&packet);

                Ok(handler(packet).into())
            }),
        );

        Ok(())
    }

    /// Register a noop packet handler.
    ///
    /// Like [`PacketHandler::register`], a duplicate registration returns the
    /// exact header and does not replace the existing handler.
    pub fn register_noop<Packet>(&mut self) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
    {
        if self.handlers.contains_key(&Packet::HEADER) {
            return Err(DuplicateHandlerError::new(Packet::HEADER));
        }

        let packet_callback = self.packet_callback.clone();
        self.metadata.insert(Packet::HEADER, PacketMetadata {
            is_variable_length: Packet::IS_VARIABLE_LENGTH,
            expected_size: Packet::EXPECTED_SIZE,
        });
        self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                packet_callback.incoming_packet(&packet);

                Ok(Output::default())
            }),
        );

        Ok(())
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

#[cfg(test)]
mod tests {
    use crate::handler::{DuplicateHandlerError, NoPacketCallback, PacketHandler};
    use crate::{LoginServerLoginSuccessPacket, Packet, PacketHeader, RequestServerTickPacket};

    #[test]
    fn duplicate_registration_returns_the_exact_header() {
        let mut handler: PacketHandler<(), NoPacketCallback> = PacketHandler::with_callback(NoPacketCallback);

        handler
            .register(|_: LoginServerLoginSuccessPacket| ())
            .expect("first registration must succeed");

        let error = handler
            .register(|_: LoginServerLoginSuccessPacket| ())
            .expect_err("second registration of the same header must be rejected");

        assert_eq!(error.packet_header, LoginServerLoginSuccessPacket::HEADER);
        assert_eq!(
            error.to_string(),
            format!(
                "duplicate packet handler registration for header 0x{:04X}",
                LoginServerLoginSuccessPacket::HEADER.0
            )
        );
    }

    #[test]
    fn duplicate_registration_does_not_replace_the_first_handler() {
        let mut handler: PacketHandler<(), NoPacketCallback> = PacketHandler::with_callback(NoPacketCallback);

        handler
            .register(|_: LoginServerLoginSuccessPacket| ())
            .expect("first registration must succeed");
        let headers_after_first = handler.registered_headers();

        // Registering a different packet and then a duplicate of the first one
        // must leave the registration set unchanged.
        handler
            .register(|_: RequestServerTickPacket| ())
            .expect("another packet must still register");
        let error = handler
            .register(|_: LoginServerLoginSuccessPacket| ())
            .expect_err("duplicate must be rejected");

        assert_eq!(error.packet_header, LoginServerLoginSuccessPacket::HEADER);
        assert_eq!(handler.registered_headers().len(), 2);
        assert!(handler.registered_headers().contains(&LoginServerLoginSuccessPacket::HEADER));
        assert!(handler.registered_headers().contains(&RequestServerTickPacket::HEADER));
        assert_ne!(headers_after_first, handler.registered_headers());
    }

    #[test]
    fn duplicate_noop_registration_returns_the_exact_header() {
        let mut handler: PacketHandler<(), NoPacketCallback> = PacketHandler::with_callback(NoPacketCallback);

        handler
            .register_noop::<LoginServerLoginSuccessPacket>()
            .expect("first registration must succeed");

        let error = handler
            .register_noop::<LoginServerLoginSuccessPacket>()
            .expect_err("second registration of the same header must be rejected");

        assert_eq!(error.packet_header, LoginServerLoginSuccessPacket::HEADER);
    }

    #[test]
    fn registering_a_noop_over_a_live_handler_returns_the_header() {
        let mut handler: PacketHandler<(), NoPacketCallback> = PacketHandler::with_callback(NoPacketCallback);

        handler
            .register(|_: LoginServerLoginSuccessPacket| ())
            .expect("handler registration must succeed");

        let error = handler
            .register_noop::<LoginServerLoginSuccessPacket>()
            .expect_err("noop registration over an existing handler must be rejected");

        assert_eq!(error.packet_header, LoginServerLoginSuccessPacket::HEADER);
    }

    #[test]
    fn duplicate_error_implements_std_error() {
        fn assert_error<T: std::error::Error>(_: &T) {}

        let error = DuplicateHandlerError::new(PacketHeader(0x00BF));
        assert_error(&error);
        assert!(error.to_string().contains("0x00BF"));
    }
}
