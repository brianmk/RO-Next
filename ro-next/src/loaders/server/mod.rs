mod client_info;

use encoding_rs::Encoding;
#[cfg(feature = "debug")]
use ro_next_debug::logging::Timer;
use ro_next_interface::element::StateElement;
use ro_next_loaders::FileLoader;
use quick_xml::Reader;
use quick_xml::de::from_str;
use quick_xml::events::Event;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

pub use self::client_info::{ClientInfo, ClientInfoPathExt, PacketVersion, Service};
use super::GameFileLoader;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, RustState, StateElement)]
pub struct ServiceId(pub usize);

pub fn load_client_info(game_file_loader: &GameFileLoader) -> ClientInfo {
    #[cfg(feature = "debug")]
    let timer = Timer::new("read clientinfo");

    let client_info = game_file_loader
        .get("data\\sclientinfo.xml")
        .or_else(|_| game_file_loader.get("data\\clientinfo.xml"))
        .expect("failed to find clientinfo");

    let content = match get_xml_encoding(&client_info) {
        Some(encoding) => {
            let (cow, _) = encoding.decode_without_bom_handling(&client_info);
            cow
        }
        None => String::from_utf8_lossy(client_info.as_slice()),
    };

    let client_info = parse_client_info(&content);

    #[cfg(feature = "debug")]
    timer.stop();

    client_info
}

/// Parse a clientinfo document.
///
/// Official clients accept a `<loading>` block with repeated `<image>` entries
/// inside `<connection>` (CondaRO.grf ships seven of them). That shape does not
/// round-trip through serde's XML mapping (it reports a duplicate `filename`),
/// and a malformed clientinfo must not abort the client, so the block is
/// stripped and the document retried once.
fn parse_client_info(content: &str) -> ClientInfo {
    match from_str(content) {
        Ok(client_info) => client_info,
        Err(error) => {
            let sanitized = strip_loading_blocks(content);

            if sanitized != content
                && let Ok(client_info) = from_str(&sanitized)
            {
                crate::logging::log_warn!("clientinfo: ignored the <loading> screens ({error})");
                return client_info;
            }

            panic!("failed to parse clientinfo: {error}");
        }
    }
}

/// Remove every `<loading>...</loading>` block (tag names are matched case
/// insensitively). The loading-screen list has no use in the client.
fn strip_loading_blocks(content: &str) -> String {
    let lowercase = content.to_ascii_lowercase();
    let mut result = String::with_capacity(content.len());
    let mut cursor = 0;

    while let Some(start) = lowercase[cursor..].find("<loading") {
        let start = cursor + start;
        result.push_str(&content[cursor..start]);

        let Some(end) = lowercase[start..].find("</loading>") else {
            return result;
        };

        let end = start + end + "</loading>".len();
        cursor = end;
    }

    result.push_str(&content[cursor..]);
    result
}

fn get_xml_encoding(data: &[u8]) -> Option<&'static Encoding> {
    let mut reader = Reader::from_reader(data);

    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Decl(xml_declaration)) => {
                if let Some(Ok(encoding)) = xml_declaration.encoding() {
                    return Encoding::for_label(encoding.as_ref());
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => (),
        }
        buffer.clear();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{parse_client_info, strip_loading_blocks};

    const CLIENT_INFO: &str = r#"<?xml version="1.0" encoding="euc-kr" ?>
<clientinfo>
    <desc>Anaconda Ragnarok Online</desc>
    <servicetype>korea</servicetype>
    <servertype>primary</servertype>
    <connection>
        <display>Ragnarok</display>
        <address>100.40.206.179</address>
        <port>6900</port>
        <version>53</version>
        <langtype>1</langtype>
REPLACE
    </connection>
</clientinfo>
"#;

    const LOADING: &str = r#"        <loading>
            <image>loading00.jpg</image>
            <image>loading01.jpg</image>
            <image>loading02.jpg</image>
        </loading>
"#;

    #[test]
    fn loading_screens_are_stripped() {
        let content = CLIENT_INFO.replace("REPLACE", LOADING);
        let stripped = strip_loading_blocks(&content);
        assert!(!stripped.contains("loading00.jpg"));
        assert!(stripped.contains("<address>100.40.206.179</address>"));
    }

    #[test]
    fn official_loading_block_does_not_abort_the_client() {
        let content = CLIENT_INFO.replace("REPLACE", LOADING);
        let info = parse_client_info(&content);
        assert_eq!(info.services[0].port, 6900);
    }

    #[test]
    fn clientinfo_without_loading_parses_unchanged() {
        let content = CLIENT_INFO.replace("REPLACE", "");
        let info = parse_client_info(&content);
        assert_eq!(info.description.as_deref(), Some("Anaconda Ragnarok Online"));
    }
}
