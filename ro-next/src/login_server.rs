//! Resolution of the login-server address selected in the login window.
//!
//! The login window stores a [`ServiceId`], which is derived from the service
//! entry in `sclientinfo.xml`/`clientinfo.xml`. The id is persisted in the
//! interface settings (`recent_service_id`), so it can point at a service that
//! no longer exists after the client info changed between two launches. The
//! service address can also be a hostname, which makes it vulnerable to DNS
//! failures.
//!
//! Both cases used to be `unwrap()`/`expect()` calls in the input handler and
//! therefore panicked the whole client. They are resolved by pure functions in
//! this module, which return a [`LoginServerResolutionError`] that the client
//! turns into an error window (see the `InputEvent::LogIn` handler).

use std::net::{SocketAddr, ToSocketAddrs};

use ro_next_networking::SupportedPacketVersion;

use crate::FALLBACK_PACKET_VERSION;
use crate::loaders::{PacketVersion, Service, ServiceId};

/// Why the service selected in the login window cannot be used to connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginServerResolutionError {
    /// The selected service id is not present in the client info (stale
    /// persisted `recent_service_id`, or the client info changed).
    StaleServiceId { service_id: ServiceId },
    /// The service declares a `PACKET_VER` this client cannot speak.
    UnsupportedPacketVersion { packet_version: u64 },
    /// The service address could not be resolved to a socket address.
    DnsResolutionFailed { address: String, message: String },
}

impl LoginServerResolutionError {
    /// Message for the client's error window.
    ///
    /// The wording is deliberately actionable: the player can retry, pick
    /// another service or restart the client after fixing the network.
    pub fn user_message(&self) -> String {
        match self {
            Self::StaleServiceId { service_id } => format!(
                "The selected server is no longer in the client configuration (service id {}). Please restart the client or choose \
                 another server.",
                service_id.0
            ),
            Self::UnsupportedPacketVersion { packet_version } => {
                format!("Selected server has an unsupported package version: {packet_version}")
            }
            Self::DnsResolutionFailed { address, message } => {
                format!("Could not resolve the server address `{address}`: {message}. Check your internet connection and try again.")
            }
        }
    }
}

/// Everything needed to connect to the login server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginServerTarget {
    pub socket_address: SocketAddr,
    pub packet_version: SupportedPacketVersion,
}

/// Resolve the login-server address and packet version for a service id.
pub fn resolve_login_server(services: &[Service], service_id: ServiceId) -> Result<LoginServerTarget, LoginServerResolutionError> {
    let service = services
        .iter()
        .find(|service| service.service_id() == service_id)
        .ok_or(LoginServerResolutionError::StaleServiceId { service_id })?;

    let packet_version = match service.packet_version {
        Some(PacketVersion::_20220406) => SupportedPacketVersion::_20220406,
        Some(PacketVersion::Unsupported(packet_version)) => {
            return Err(LoginServerResolutionError::UnsupportedPacketVersion { packet_version });
        }
        None => FALLBACK_PACKET_VERSION,
    };

    let address = format!("{}:{}", service.address, service.port);
    let socket_address = resolve_socket_address(&address)?;

    Ok(LoginServerTarget {
        socket_address,
        packet_version,
    })
}

/// Resolve a `host:port` string to a socket address.
///
/// This is split from [`resolve_login_server`] so the DNS failure path can be
/// tested without constructing a whole client-info service.
fn resolve_socket_address(address: &str) -> Result<SocketAddr, LoginServerResolutionError> {
    let mut candidates = address
        .to_socket_addrs()
        .map_err(|error| LoginServerResolutionError::DnsResolutionFailed {
            address: address.to_owned(),
            message: error.to_string(),
        })?;

    candidates.next().ok_or_else(|| LoginServerResolutionError::DnsResolutionFailed {
        address: address.to_owned(),
        message: "the address did not resolve to a socket address".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use ro_next_networking::SupportedPacketVersion;

    use super::{LoginServerResolutionError, LoginServerTarget, resolve_login_server, resolve_socket_address};
    use crate::loaders::{PacketVersion, Service, ServiceId};

    fn service(address: &str, port: u16) -> Service {
        Service {
            address: address.to_owned(),
            port,
            ..Default::default()
        }
    }

    #[test]
    fn stale_service_id_returns_an_error_message_instead_of_panicking() {
        // The login window can hold an id from a previous client-info version.
        let services = [service("127.0.0.1", 6900)];
        let stale_service_id = ServiceId(usize::MAX);

        let error = resolve_login_server(&services, stale_service_id).expect_err("stale service id must not resolve");

        assert_eq!(error, LoginServerResolutionError::StaleServiceId {
            service_id: stale_service_id
        });
        let message = error.user_message();
        assert!(message.contains("no longer in the client configuration"), "{message}");
    }

    #[test]
    fn empty_service_list_is_a_stale_service_id_not_a_panic() {
        let error = resolve_login_server(&[], ServiceId(0)).expect_err("an empty service list cannot resolve");
        assert!(matches!(error, LoginServerResolutionError::StaleServiceId { .. }));
    }

    #[test]
    fn dns_resolution_failure_returns_an_error_message_instead_of_panicking() {
        // `.invalid` is reserved by RFC 2606 and can never resolve.
        let error = resolve_socket_address("ro-next-dns-failure-test.invalid:6900")
            .expect_err("an invalid hostname must not resolve to a socket address");

        let LoginServerResolutionError::DnsResolutionFailed { address, .. } = &error else {
            panic!("expected a DNS resolution error, got {error:?}");
        };
        assert_eq!(address, "ro-next-dns-failure-test.invalid:6900");

        let message = error.user_message();
        assert!(message.contains("Could not resolve the server address"), "{message}");
        assert!(message.contains("ro-next-dns-failure-test.invalid:6900"), "{message}");
    }

    #[test]
    fn malformed_socket_address_returns_a_dns_error_instead_of_panicking() {
        // A port outside of `u16` range is not a valid socket address.
        let error = resolve_socket_address("127.0.0.1:not-a-port").expect_err("malformed address must not resolve");
        assert!(matches!(error, LoginServerResolutionError::DnsResolutionFailed { .. }));
    }

    #[test]
    fn numeric_address_resolves_without_dns() {
        let resolved = resolve_socket_address("127.0.0.1:6900").expect("localhost must resolve");
        assert_eq!(resolved.port(), 6900);
        assert!(resolved.ip().is_loopback());
    }

    #[test]
    fn supported_service_resolves_to_the_expected_target() {
        let mut supported = service("127.0.0.1", 6900);
        supported.packet_version = Some(PacketVersion::_20220406);
        let services = [supported];

        let target = resolve_login_server(&services, services[0].service_id()).expect("a supported local service must resolve");
        assert_eq!(target, LoginServerTarget {
            socket_address: "127.0.0.1:6900".parse().expect("valid test address"),
            packet_version: SupportedPacketVersion::_20220406,
        });
    }

    #[test]
    fn missing_packet_version_uses_the_fallback_version() {
        let services = [service("127.0.0.1", 6900)];
        let target = resolve_login_server(&services, services[0].service_id()).expect("a local service must resolve");
        assert_eq!(target.packet_version, crate::FALLBACK_PACKET_VERSION);
    }

    #[test]
    fn unsupported_packet_version_returns_an_error_message() {
        let mut unsupported = service("127.0.0.1", 6900);
        unsupported.packet_version = Some(PacketVersion::Unsupported(20190605));
        let services = [unsupported];

        let error = resolve_login_server(&services, services[0].service_id()).expect_err("unsupported packet version must not resolve");
        assert_eq!(error, LoginServerResolutionError::UnsupportedPacketVersion {
            packet_version: 20190605
        });
        assert!(error.user_message().contains("unsupported package version"));
    }
}
