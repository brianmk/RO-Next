//! Headless parser check for CondaRO's 2019 character-list packets.
//! Usage: cargo run -p ragnarok-packets --example parse_char_list -- <file>
//! The file must start with the packet header (0x006B or 0x099D) directly.
use ragnarok_bytes::ByteReader;
use ragnarok_packets::handler::{HandlerResult, NoPacketCallback, PacketHandler};
use ragnarok_packets::{CharacterInfoPerPagePacket, CharacterListPacket};

fn main() {
    let path = std::env::args().nth(1).expect("usage: parse_char_list <file>");
    let data = std::fs::read(&path).expect("read file");
    let mut handler = PacketHandler::<Vec<String>, NoPacketCallback>::with_callback(NoPacketCallback);
    handler
        .register(|packet: CharacterListPacket| {
            packet
                .character_information
                .into_iter()
                .map(|character| format!("0x006B: {}", character.name))
                .collect::<Vec<_>>()
        })
        .unwrap();
    handler
        .register(|packet: CharacterInfoPerPagePacket| {
            packet
                .character_information
                .into_iter()
                .map(|character| format!("0x099D: {}", character.name))
                .collect::<Vec<_>>()
        })
        .unwrap();
    let mut reader = ByteReader::without_metadata(&data);
    let mut names = Vec::new();
    while !reader.is_empty() {
        match handler.process_one(&mut reader) {
            HandlerResult::Ok(mut found) => names.append(&mut found),
            HandlerResult::PacketCutOff | HandlerResult::UnhandledPacket | HandlerResult::InternalError(_) => {
                println!(
                    "parse stopped at offset {} ({} bytes left)",
                    reader.get_offset(),
                    reader.remaining_bytes().len()
                );
                break;
            }
        }
    }
    println!("parsed {} characters: {:?}", names.len(), names);
}
