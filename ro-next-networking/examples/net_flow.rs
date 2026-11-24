//! Headless end-to-end network-flow probe for the CondaRO 2019 rAthena server.
//! Login -> character server -> character list -> character select -> map
//! server. Used to verify ro-next's patched 2019 packet handling without the
//! GUI.
use std::time::{Duration, Instant};

use ro_next_networking::{LoginServerLoginData, NetworkEvent, NetworkingSystem, SupportedPacketVersion};

fn main() {
    let version = SupportedPacketVersion::_20220406;
    let (mut net, mut events) = NetworkingSystem::spawn();
    let address = "192.168.1.33:6900".parse().unwrap();
    net.connect_to_login_server(version, address, "slavecake", "slavecake");
    println!("connecting to login server {address} ...");

    let mut login_data: Option<LoginServerLoginData> = None;
    let mut char_list_done = false;
    let deadline = Instant::now() + Duration::from_secs(45);

    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(150));
        net.get_events(&mut events);
        for event in events.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data: data,
                } => {
                    println!("LOGIN OK: {} char server(s):", character_servers.len());
                    for server in &character_servers {
                        println!(
                            "  - {} {:?}:{} users={}",
                            server.server_name, server.server_ip, server.server_port, server.user_count
                        );
                    }
                    login_data = Some(data);
                    if let Some(server) = character_servers.first().cloned() {
                        net.connect_to_character_server(version, &data, server);
                    } else {
                        println!("FAIL: no character server in login response");
                        return;
                    }
                }
                NetworkEvent::AccountId { account_id } => println!("char server sends account id echo: {account_id:?}"),
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    println!("CHAR SERVER OK: normal_slot_count={normal_slot_count}");
                    let _ = net.request_character_list();
                }
                NetworkEvent::CharacterList { characters } => {
                    println!("CHARACTER LIST OK: {} character(s)", characters.len());
                    for character in &characters {
                        println!(
                            "  slot={} {} job={:?} base={} hp={}/{} sp={}/{} map={}",
                            character.character_number,
                            character.name,
                            character.job_id,
                            character.base_level,
                            character.health_points,
                            character.maximum_health_points,
                            character.spell_points,
                            character.maximum_spell_points,
                            character.map_name
                        );
                    }
                    char_list_done = true;
                    if let Some(character) = characters.first() {
                        let slot = character.character_number as usize;
                        let _ = net.select_character(slot);
                        println!("selecting character slot {slot} ...");
                    }
                }
                NetworkEvent::CharacterSelected { login_data: cd } => {
                    println!(
                        "CHARACTER SELECTED: map server {}:{} char={:?}",
                        cd.server_ip, cd.server_port, cd.character_id
                    );
                    if let Some(data) = login_data {
                        net.connect_to_map_server(version, &data, cd);
                        println!("connecting to map server ...");
                    }
                }
                NetworkEvent::InitialStats { .. } => {
                    println!("MAP SERVER LOGIN OK: received InitialStats");
                    return;
                }
                NetworkEvent::ChangeMap { map_name, position } => {
                    println!("MAP CHANGE: {map_name} {position:?}");
                }
                NetworkEvent::LoginServerConnectionFailed { reason, message } => {
                    println!("FAIL login: {reason:?} {message}");
                    return;
                }
                NetworkEvent::CharacterServerConnectionFailed { reason, message } => {
                    println!("FAIL char server: {reason:?} {message}");
                    return;
                }
                NetworkEvent::CharacterSelectionFailed { reason, message } => {
                    println!("FAIL char select: {reason:?} {message}");
                    return;
                }
                NetworkEvent::LoginServerDisconnected { reason } => println!("login disconnected: {reason:?}"),
                NetworkEvent::CharacterServerDisconnected { reason } => println!("char disconnected: {reason:?}"),
                NetworkEvent::MapServerDisconnected { reason } => println!("map disconnected: {reason:?}"),
                other => println!("event: {}", event_name(&other)),
            }
        }
    }
    println!("TIMEOUT (char_list_done={char_list_done})");
}

fn event_name(event: &NetworkEvent) -> &'static str {
    match event {
        NetworkEvent::UpdateClientTick { .. } => "UpdateClientTick",
        NetworkEvent::AddEntity { .. } => "AddEntity",
        NetworkEvent::RemoveEntity { .. } => "RemoveEntity",
        NetworkEvent::PlayerMove { .. } => "PlayerMove",
        NetworkEvent::EntityMove { .. } => "EntityMove",
        NetworkEvent::ResurrectPlayer { .. } => "ResurrectPlayer",
        NetworkEvent::PlayerStandUp { .. } => "PlayerStandUp",
        NetworkEvent::DisplayEmotion { .. } => "DisplayEmotion",
        _ => "other",
    }
}
