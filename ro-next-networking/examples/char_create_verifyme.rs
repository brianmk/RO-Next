use std::time::{Duration, Instant};

use ro_next_networking::{LoginServerLoginData, NetworkEvent, NetworkingSystem, SupportedPacketVersion};
fn main() {
    let version = SupportedPacketVersion::_20220406;
    let (mut net, mut events) = NetworkingSystem::spawn();
    net.connect_to_login_server(version, "192.168.1.33:6900".parse().unwrap(), "verifyme", "verifyme");
    let _login: Option<LoginServerLoginData> = None;
    let mut created = false;
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(150));
        net.get_events(&mut events);
        for event in events.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    if let Some(server) = character_servers.first().cloned() {
                        net.connect_to_character_server(version, &login_data, server);
                        println!("login ok -> char server");
                    }
                }
                NetworkEvent::CharacterServerConnected { .. } => {
                    let _ = net.request_character_list();
                }
                NetworkEvent::CharacterList { characters } => {
                    println!("characters: {}", characters.len());
                    if characters.is_empty() {
                        let _ = net.create_character(0, "VerifyBot".to_string());
                    } else {
                        println!("already has character");
                        created = true;
                    }
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    println!(
                        "CHAR CREATED: {} slot={}",
                        character_information.name, character_information.character_number
                    );
                    created = true;
                }
                NetworkEvent::CharacterCreationFailed { reason, message } => {
                    println!("CREATE FAILED: {reason:?} {message}");
                    return;
                }
                NetworkEvent::LoginServerConnectionFailed { reason, message } => {
                    println!("LOGIN FAIL: {reason:?} {message}");
                    return;
                }
                _ => {}
            }
        }
        if created {
            return;
        }
    }
}
