//! Shared network-safety helpers for outbound requests.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Returns `true` only for genuinely public, routable IP addresses.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_ipv4(v4),
        IpAddr::V6(v6) => is_public_ipv6(v6),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();

    if a == 0 || a == 10 || a == 127 {
        return false;
    }
    if a == 100 && (64..=127).contains(&b) {
        return false;
    }
    if a == 169 && b == 254 {
        return false;
    }
    if a == 172 && (16..=31).contains(&b) {
        return false;
    }
    if a == 192 && b == 0 && (c == 0 || c == 2) {
        return false;
    }
    if a == 192 && b == 168 {
        return false;
    }
    if a == 198 && (b == 18 || b == 19) {
        return false;
    }
    if a == 198 && b == 51 && c == 100 {
        return false;
    }
    if a == 203 && b == 0 && c == 113 {
        return false;
    }
    if a >= 224 {
        return false;
    }

    true
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() {
        return false;
    }
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_ipv4(v4);
    }

    let segments = ip.segments();
    if segments[..6].iter().all(|&segment| segment == 0) {
        let v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return is_public_ipv4(v4);
    }

    let first = segments[0];
    if (first & 0xffc0) == 0xfe80 || (first & 0xfe00) == 0xfc00 || (first & 0xff00) == 0xff00 {
        return false;
    }
    if first == 0x2001 && segments[1] == 0x0db8 {
        return false;
    }
    if first == 0x0100 && segments[1..4].iter().all(|&segment| segment == 0) {
        return false;
    }

    true
}

/// Returns `true` for hostnames that explicitly identify the local machine.
pub(crate) fn is_blocked_hostname(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized == "localhost.localdomain"
        || normalized.ends_with(".localdomain")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(raw: &str) -> IpAddr {
        raw.parse().unwrap()
    }

    #[test]
    fn rejects_non_public_ipv4_ranges() {
        for raw in [
            "0.0.0.0",
            "10.0.0.1",
            "127.0.0.1",
            "100.64.0.1",
            "169.254.0.1",
            "172.16.0.1",
            "192.0.0.1",
            "192.0.2.1",
            "192.168.0.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            assert!(!is_public_ip(ip(raw)), "{raw} must be blocked");
        }
    }

    #[test]
    fn accepts_public_ipv4_boundary_examples() {
        for raw in [
            "8.8.8.8",
            "1.1.1.1",
            "172.15.255.255",
            "172.32.0.1",
            "100.63.255.255",
            "100.128.0.0",
        ] {
            assert!(is_public_ip(ip(raw)), "{raw} must be allowed");
        }
    }

    #[test]
    fn rejects_non_public_ipv6_ranges_and_embedded_private_ipv4() {
        for raw in [
            "::",
            "::1",
            "fe80::1",
            "fc00::1",
            "fd00::1",
            "ff02::1",
            "2001:db8::1",
            "100::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::192.168.1.1",
        ] {
            assert!(!is_public_ip(ip(raw)), "{raw} must be blocked");
        }
    }

    #[test]
    fn accepts_public_ipv6_and_embedded_public_ipv4() {
        for raw in [
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
            "::ffff:8.8.8.8",
        ] {
            assert!(is_public_ip(ip(raw)), "{raw} must be allowed");
        }
    }

    #[test]
    fn blocks_local_hostname_forms() {
        for host in [
            "localhost",
            "LOCALHOST",
            "foo.localhost",
            "localhost.localdomain",
            "box.localdomain",
        ] {
            assert!(is_blocked_hostname(host));
        }
        assert!(!is_blocked_hostname("example.com"));
    }
}
