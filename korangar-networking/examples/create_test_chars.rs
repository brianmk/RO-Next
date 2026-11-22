use std::time::{Duration, Instant};

use korangar_networking::{NetworkEvent, NetworkingSystem, SupportedPacketVersion};

fn create_one(user: &str, pass: &str, char_name: &str) {
    let version = SupportedPacketVersion::_20220406;
    let (mut net, mut events) = NetworkingSystem::spawn();
    net.connect_to_login_server(version, "192.168.1.33:6900".parse().unwrap(), user, pass);
    let mut done = false;
    let deadline = Instant::now() + Duration::from_secs(25);
    while Instant::now() < deadline && !done {
        std::thread::sleep(Duration::from_millis(150));
        net.get_events(&mut events);
        for event in events.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    // Rewrite WAN-advertised char server to LAN for this host.
                    let mut server = character_servers[0].clone();
                    server.server_ip = ragnarok_packets::ServerAddress([192, 168, 1, 33]);
                    net.connect_to_character_server(version, &login_data, server);
                }
                NetworkEvent::CharacterServerConnected { .. } => {
                    let _ = net.request_character_list();
                }
                NetworkEvent::CharacterList { characters } => {
                    if characters.is_empty() {
                        let _ = net.create_character(0, char_name.to_string());
                    } else {
                        println!("{user}: already has {}", characters.len());
                        done = true;
                    }
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    println!(
                        "{user}: created {} slot={}",
                        character_information.name, character_information.character_number
                    );
                    done = true;
                }
                NetworkEvent::CharacterCreationFailed { reason, message } => {
                    println!("{user}: CREATE FAILED {reason:?} {message}");
                    done = true;
                }
                NetworkEvent::LoginServerConnectionFailed { reason, message } => {
                    println!("{user}: LOGIN FAIL {reason:?} {message}");
                    done = true;
                }
                NetworkEvent::CharacterServerConnectionFailed { reason, message } => {
                    println!("{user}: CHAR FAIL {reason:?} {message}");
                    done = true;
                }
                _ => {}
            }
        }
    }
    if !done {
        println!("{user}: TIMEOUT");
    }
    std::thread::sleep(Duration::from_millis(300));
}

fn main() {
    for i in 6..=6 {
        create_one(&format!("verify{i}"), "verifypass", &format!("VerifyBot{i}"));
        std::thread::sleep(Duration::from_millis(800));
    }
}
