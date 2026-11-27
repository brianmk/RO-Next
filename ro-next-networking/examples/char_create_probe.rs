//! Headless probe: login -> char server -> create a character -> expect
//! success. Verifies ro-next's 2019 packet support (0x0436 map login is not
//! needed here).
use std::time::{Duration, Instant};

use ro_next_networking::{NetworkEvent, NetworkingSystem, SupportedPacketVersion};

fn main() {
    let version = SupportedPacketVersion::_20220406;
    let (mut net, mut events) = NetworkingSystem::spawn();
    net.connect_to_login_server(version, "192.168.1.33:6900".parse().unwrap(), "ro-nexttest", "ro-next");
    println!("logging in as ro-nexttest ...");

    let mut created = false;
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(120));
        net.get_events(&mut events);
        for event in events.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data: data,
                } => {
                    if let Some(server) = character_servers.first().cloned() {
                        net.connect_to_character_server(version, &data, server);
                        println!("connected login, going to char server");
                    }
                }
                NetworkEvent::CharacterServerConnected { .. } => {
                    println!("char server connected, requesting character list");
                    let _ = net.request_character_list();
                }
                NetworkEvent::CharacterList { characters } => {
                    println!("character list: {} character(s)", characters.len());
                    println!("creating character 'RO-NextTest' in slot 0 ...");
                    let _ = net.create_character(0, "RO-NextTest".to_string());
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    println!(
                        "CHARACTER CREATED OK: name={} slot={} job={:?} base={}",
                        character_information.name,
                        character_information.character_number,
                        character_information.job_id,
                        character_information.base_level
                    );
                    created = true;
                }
                NetworkEvent::CharacterCreationFailed { reason, message } => {
                    println!("CHARACTER CREATION FAILED: {reason:?} {message}");
                    return;
                }
                NetworkEvent::LoginServerConnectionFailed { reason, message } => {
                    println!("login failed: {reason:?} {message}");
                    return;
                }
                NetworkEvent::CharacterServerConnectionFailed { reason, message } => {
                    println!("char server failed: {reason:?} {message}");
                    return;
                }
                _ => {}
            }
        }
        if created {
            std::thread::sleep(Duration::from_millis(300));
            return;
        }
    }
    println!("TIMEOUT");
}
