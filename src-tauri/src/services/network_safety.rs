//! Shared network-safety helpers for outbound requests.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use crate::error::AppError;

/// Maximum 3xx hops a caller may follow after re-checking each Location.
pub const MAX_REDIRECTS: usize = 5;

/// User-Agent attached to every DNS-pinned outbound client.
const USER_AGENT: &str = "PromptHub/1.0";

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
        return is_public_ipv4(ipv4_from_tail(segments));
    }
    // 6to4 2002::/16 embeds IPv4 in bits 16..48.
    if segments[0] == 0x2002 {
        return is_public_ipv4(Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            (segments[1] & 0xff) as u8,
            (segments[2] >> 8) as u8,
            (segments[2] & 0xff) as u8,
        ));
    }
    // NAT64 well-known prefix 64:ff9b::/96 embeds IPv4 in the last 32 bits.
    if segments[0] == 0x64
        && segments[1] == 0xff9b
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
    {
        return is_public_ipv4(ipv4_from_tail(segments));
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

fn ipv4_from_tail(segments: [u16; 8]) -> Ipv4Addr {
    Ipv4Addr::new(
        (segments[6] >> 8) as u8,
        (segments[6] & 0xff) as u8,
        (segments[7] >> 8) as u8,
        (segments[7] & 0xff) as u8,
    )
}

/// Returns `true` when `left` and `right` share scheme and host (not port).
pub fn same_http_origin(left: &reqwest::Url, right: &reqwest::Url) -> bool {
    left.scheme() == right.scheme() && left.host() == right.host()
}

/// Returns `true` for hostnames that explicitly identify the local machine.
pub(crate) fn is_blocked_hostname(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized == "localhost.localdomain"
        || normalized.ends_with(".localdomain")
}

fn ssrf_blocked_host(host: &str, message: String) -> AppError {
    crate::logging::event(
        crate::logging::Level::Warn,
        "ssrf",
        format!("blocked host `{host}`"),
    );
    AppError::ssrf_blocked(message)
}

/// Validates and DNS-pins one outbound HTTP(S) hop before it is contacted.
/// Callers that follow redirects must call this again for every target.
///
/// When `allow_private_network` is false, loopback, RFC1918, link-local,
/// metadata, and blocked hostnames such as `localhost` return `SSRF_BLOCKED`
/// with no TCP connect. When true, those addresses may be attempted; the
/// scheme must still be http/https, and the client still DNS-pins with
/// `redirect(Policy::none())`.
pub async fn prepare_public_url(
    raw: &str,
    timeout: Duration,
    allow_private_network: bool,
) -> Result<(reqwest::Url, reqwest::Client), AppError> {
    let url = reqwest::Url::parse(raw)
        .map_err(|e| AppError::validation(format!("invalid URL `{raw}`: {e}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        crate::logging::event(
            crate::logging::Level::Warn,
            "ssrf",
            format!("blocked scheme `{}`", url.scheme()),
        );
        return Err(AppError::ssrf_blocked(
            "only HTTP and HTTPS outbound URLs are allowed",
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| AppError::validation("provider endpoint has no host"))?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    if !allow_private_network && is_blocked_hostname(&host) {
        return Err(ssrf_blocked_host(
            &host,
            format!("host `{host}` names the local machine"),
        ));
    }
    let port = url
        .port_or_known_default()
        .unwrap_or(if url.scheme() == "https" { 443 } else { 80 });
    let ips = if let Ok(ip) = host.parse::<IpAddr>() {
        if !address_permitted(ip, allow_private_network) {
            return Err(ssrf_blocked_host(
                &host,
                format!("host `{host}` resolves to a non-public address"),
            ));
        }
        vec![ip]
    } else {
        let resolved = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|e| AppError::network(format!("failed to resolve host `{host}`: {e}")))?;
        let mut ips = Vec::new();
        for address in resolved {
            if !address_permitted(address.ip(), allow_private_network) {
                return Err(ssrf_blocked_host(
                    &host,
                    format!("host `{host}` resolves to a non-public address"),
                ));
            }
            if !ips.contains(&address.ip()) {
                ips.push(address.ip());
            }
        }
        if ips.is_empty() {
            return Err(AppError::network(format!(
                "host `{host}` did not resolve to an address"
            )));
        }
        ips
    };
    let addrs: Vec<SocketAddr> = ips
        .into_iter()
        .map(|ip| SocketAddr::new(ip, port))
        .collect();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .resolve_to_addrs(&host, &addrs)
        .build()
        .map_err(|e| AppError::network(format!("failed to build provider client: {e}")))?;
    Ok((url, client))
}

fn address_permitted(ip: IpAddr, allow_private_network: bool) -> bool {
    allow_private_network || is_public_ip(ip)
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
            "2002:a00:1::",
            "2002:7f00:1::",
            "64:ff9b::a00:1",
            "64:ff9b::7f00:1",
        ] {
            assert!(!is_public_ip(ip(raw)), "{raw} must be blocked");
        }
    }

    #[test]
    fn six_to_four_embedded_private_ipv4_is_non_public() {
        // 2002:a00:1:: embeds 10.0.0.1 in bits 16..48.
        assert!(!is_public_ip(ip("2002:a00:1::")));
        assert!(!is_public_ip(ip("2002:0a00:0001::")));
    }

    #[test]
    fn accepts_public_ipv6_and_embedded_public_ipv4() {
        for raw in [
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
            "::ffff:8.8.8.8",
            "2002:808:808::",
            "64:ff9b::808:808",
        ] {
            assert!(is_public_ip(ip(raw)), "{raw} must be allowed");
        }
    }

    #[test]
    fn same_http_origin_compares_scheme_and_host_only() {
        let left = reqwest::Url::parse("https://api.example.com:8443/v1").unwrap();
        let same_host = reqwest::Url::parse("https://api.example.com/v2").unwrap();
        let other_host = reqwest::Url::parse("https://evil.example/v1").unwrap();
        let other_scheme = reqwest::Url::parse("http://api.example.com/v1").unwrap();
        assert!(same_http_origin(&left, &same_host));
        assert!(!same_http_origin(&left, &other_host));
        assert!(!same_http_origin(&left, &other_scheme));
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

    #[tokio::test]
    async fn public_url_precheck_blocks_local_provider_endpoints() {
        let error = prepare_public_url(
            "http://localhost:8080/v1/chat",
            Duration::from_secs(1),
            false,
        )
        .await
        .unwrap_err();
        assert_eq!(error.code_str(), "SSRF_BLOCKED");
    }

    #[tokio::test]
    async fn public_url_precheck_blocks_loopback_literal_without_connect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accepted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = accepted.clone();
        let server = tokio::spawn(async move {
            if listener.accept().await.is_ok() {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
        let error = prepare_public_url(
            &format!("http://127.0.0.1:{}/", addr.port()),
            Duration::from_secs(1),
            false,
        )
        .await
        .unwrap_err();
        assert_eq!(error.code_str(), "SSRF_BLOCKED");
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !accepted.load(std::sync::atomic::Ordering::SeqCst),
            "loopback must not be contacted"
        );
        server.abort();
    }

    #[tokio::test]
    async fn prepare_allows_private_literal_when_private_network_enabled() {
        let result = prepare_public_url("http://127.0.0.1:9/", Duration::from_secs(1), true).await;
        assert!(
            result.is_ok(),
            "allow-private must pin a loopback literal without blocking: {result:?}"
        );
    }

    #[tokio::test]
    async fn prepare_allows_localhost_hostname_when_private_network_enabled() {
        let result = prepare_public_url("http://localhost:9/", Duration::from_secs(1), true).await;
        assert!(
            result.is_ok(),
            "allow-private must accept localhost: {result:?}"
        );
    }

    #[tokio::test]
    async fn prepare_still_rejects_non_http_when_private_network_enabled() {
        let error = prepare_public_url("ftp://127.0.0.1/", Duration::from_secs(1), true)
            .await
            .unwrap_err();
        assert_eq!(error.code_str(), "SSRF_BLOCKED");
    }
}
