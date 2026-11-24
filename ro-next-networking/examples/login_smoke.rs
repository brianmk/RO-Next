use std::time::{Duration, Instant};

use ro_next_networking::{NetworkEvent, NetworkingSystem, SupportedPacketVersion};
fn main() {
    let version = SupportedPacketVersion::_20220406;
    let (mut net, mut events) = NetworkingSystem::spawn();
    net.connect_to_login_server(version, "192.168.1.33:6900".parse().unwrap(), "verifyme", "verifyme");
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(150));
        net.get_events(&mut events);
        for event in events.drain() {
            match event {
                NetworkEvent::LoginServerConnected { character_servers, .. } => {
                    println!("LOGIN_OK servers={}", character_servers.len());
                    return;
                }
                NetworkEvent::LoginServerConnectionFailed { reason, message } => {
                    println!("LOGIN_FAIL {reason:?} {message}");
                    return;
                }
                _ => {}
            }
        }
    }
    println!("TIMEOUT");
}
