//! Headless probe for the login -> char-server handoff ("select server" click).
//! Uses a dedicated verify account. Verifies that:
//!  1) login succeeds,
//!  2) connect_to_character_server establishes the 2019 CH_ENTER handshake
//!     (account id echo + character list),
//!  3) a *second* connect_to_character_server call (stale connection / repeated
//!     click) still results in a fresh, working character server session.
use std::time::{Duration, Instant};

use korangar_networking::{LoginServerLoginData, NetworkEvent, NetworkingSystem, SupportedPacketVersion};
use ragnarok_packets::CharacterServerInformation;

fn main() {
    let version = SupportedPacketVersion::_20220406;
    let (mut net, mut events) = NetworkingSystem::spawn();
    let address = "192.168.1.33:6900".parse().unwrap();
    let username = std::env::args().nth(1).unwrap_or_else(|| "verify1".to_string());
    let password = std::env::args().nth(2).unwrap_or_else(|| "verifypass".to_string());
    net.connect_to_login_server(version, address, &username, &password);
    println!("connecting to login server {address} ...");

    let mut login_data: Option<LoginServerLoginData> = None;
    let mut char_server: Option<CharacterServerInformation> = None;
    let mut char_connected = 0;
    let mut char_lists = 0;
    let mut reconnect_sent = false;
    let deadline = Instant::now() + Duration::from_secs(40);

    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(150));
        net.get_events(&mut events);
        for event in events.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data: data,
                } => {
                    println!("LOGIN OK: {} char server(s)", character_servers.len());
                    login_data = Some(data);
                    if let Some(server) = character_servers.first().cloned() {
                        println!("  using {:?}:{}", server.server_ip, server.server_port);
                        char_server = Some(server.clone());
                        net.connect_to_character_server(version, &data, server);
                    }
                }
                NetworkEvent::AccountId { account_id } => println!("CHAR ACCOUNT-ID ECHO OK: {account_id:?}"),
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    char_connected += 1;
                    println!("CHAR SERVER CONNECTED (#{char_connected}) slots={normal_slot_count}");
                    let _ = net.request_character_list();
                }
                NetworkEvent::CharacterList { characters } => {
                    char_lists += 1;
                    println!("CHARACTER LIST OK (#{char_lists}): {} char(s)", characters.len());
                    for character in &characters {
                        println!(
                            "  - {} job={:?} base={}",
                            character.name, character.job_id, character.base_level
                        );
                    }

                    if !reconnect_sent {
                        reconnect_sent = true;
                        // Simulate a repeated "select server" click while the previous
                        // connection is still registered: the second connect must replace
                        // the stale connection instead of being silently ignored.
                        if let (Some(data), Some(server)) = (login_data.clone(), char_server.clone()) {
                            println!("--- second connect_to_character_server (stale-connection test) ---");
                            net.connect_to_character_server(version, &data, server);
                        }
                    } else {
                        println!("RESULT: OK — reconnect after a stale connection produced a character list");
                        return;
                    }
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => println!("CHAR CONNECT FAILED: {message}"),
                NetworkEvent::CharacterServerDisconnected { reason } => println!("CHAR DISCONNECTED: {reason:?}"),
                other => println!("EVENT: {other:?}"),
            }
        }
    }
    println!("RESULT: TIMEOUT char_connected={char_connected} char_lists={char_lists}");
}
