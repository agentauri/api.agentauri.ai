//! IP Address Extraction with Proxy Support
//!
//! This module provides utilities for extracting the client's real IP address
//! from HTTP requests, accounting for proxies, CDNs, and load balancers.
//!
//! # Security Considerations
//!
//! - Only trusts `X-Forwarded-For` and `X-Real-IP` if coming from trusted proxies
//! - Validates IP addresses to prevent header spoofing
//! - Falls back to connection peer address if headers are invalid
//! - **Parses X-Forwarded-For from right-to-left** to prevent IP spoofing attacks
//!   (attacker cannot inject fake IPs that will be trusted)
//!
//! # Trusted Proxies
//!
//! Configure via `TRUSTED_PROXIES` environment variable (comma-separated):
//! ```bash
//! TRUSTED_PROXIES=127.0.0.1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16
//! ```
//!
//! # X-Forwarded-For Parsing
//!
//! The header format is: `client, proxy1, proxy2, ..., proxyN`
//!
//! **Security**: We parse from right-to-left, trusting only proxies we know.
//! The first untrusted IP (walking backwards) is the client IP.
//!
//! Example: `X-Forwarded-For: attacker-spoofed, real-client, trusted-proxy1`
//! - Walking from right: trusted-proxy1 (trusted, skip), real-client (untrusted, return)
//! - This prevents attacker from spoofing their IP by prepending fake IPs
//!
//! Rust guideline compliant 2025-01-28

use actix_web::HttpRequest;
use std::env;
use std::net::IpAddr;
use std::str::FromStr;
use tracing::{debug, warn};

/// Extract the client's IP address from the request
///
/// This function checks headers in the following order:
/// 1. `X-Forwarded-For` (if from trusted proxy)
/// 2. `X-Real-IP` (if from trusted proxy)
/// 3. Connection peer address (fallback)
///
/// # Arguments
///
/// * `req` - The HTTP request
///
/// # Returns
///
/// The client's IP address as a string, or "unknown" if it cannot be determined
pub fn extract_ip(req: &HttpRequest) -> String {
    // Get connection peer address (always available)
    let peer_ip = req
        .connection_info()
        .peer_addr()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Check if the peer is a trusted proxy
    if !is_trusted_proxy(&peer_ip) {
        debug!(peer_ip = %peer_ip, "Using peer address (not from trusted proxy)");
        return peer_ip;
    }

    // Peer is trusted, check X-Forwarded-For header
    // SECURITY: Parse from right-to-left to prevent IP spoofing
    // The rightmost untrusted IP is the real client
    if let Some(forwarded_for) = req.headers().get("X-Forwarded-For") {
        if let Ok(value) = forwarded_for.to_str() {
            // X-Forwarded-For format: "client, proxy1, proxy2, ..., proxyN"
            // Walk backwards through the chain, skipping trusted proxies
            // The first untrusted IP (from right) is the real client
            let ips: Vec<&str> = value.split(',').map(|s| s.trim()).collect();

            for ip in ips.iter().rev() {
                if !is_valid_ip(ip) {
                    // Skip invalid IPs but log a warning
                    warn!(ip = %ip, "Invalid IP in X-Forwarded-For chain, skipping");
                    continue;
                }

                // If this IP is not a trusted proxy, it's the client
                if !is_trusted_proxy(ip) {
                    debug!(
                        client_ip = %ip,
                        peer_ip = %peer_ip,
                        chain = %value,
                        "Extracted client IP from X-Forwarded-For (right-to-left)"
                    );
                    return (*ip).to_string();
                }
                // Otherwise, continue to the next IP (to the left)
            }

            // If all IPs in the chain are trusted (unlikely but possible),
            // fall through to X-Real-IP or peer address
            debug!(
                chain = %value,
                "All IPs in X-Forwarded-For are trusted proxies"
            );
        }
    }

    // Check X-Real-IP header (used by some proxies like Nginx)
    if let Some(real_ip) = req.headers().get("X-Real-IP") {
        if let Ok(value) = real_ip.to_str() {
            let value = value.trim();
            if is_valid_ip(value) {
                debug!(
                    client_ip = %value,
                    peer_ip = %peer_ip,
                    "Using X-Real-IP header"
                );
                return value.to_string();
            }
        }
    }

    // Fallback to peer address
    debug!(peer_ip = %peer_ip, "Using peer address (no valid headers)");
    peer_ip
}

/// Check if an IP address is a trusted proxy
///
/// Trusted proxies are configured via the `TRUSTED_PROXIES` environment variable.
/// Default trusted IPs include common private network ranges.
///
/// # Arguments
///
/// * `ip` - The IP address to check
///
/// # Returns
///
/// `true` if the IP is trusted, `false` otherwise
fn is_trusted_proxy(ip: &str) -> bool {
    // Get trusted proxies from environment variable
    let trusted_proxies = env::var("TRUSTED_PROXIES").unwrap_or_else(|_| {
        // Default: Trust localhost and private network ranges
        "127.0.0.1,::1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,fc00::/7".to_string()
    });

    let ip_addr = match IpAddr::from_str(ip) {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    for trusted in trusted_proxies.split(',') {
        let trusted = trusted.trim();

        // Handle CIDR notation (e.g., "10.0.0.0/8")
        if trusted.contains('/') {
            if ip_in_cidr(&ip_addr, trusted) {
                return true;
            }
        } else {
            // Exact IP match
            if let Ok(trusted_addr) = IpAddr::from_str(trusted) {
                if ip_addr == trusted_addr {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if an IP is within a CIDR range
///
/// # Arguments
///
/// * `ip` - The IP address to check
/// * `cidr` - The CIDR notation (e.g., "10.0.0.0/8")
///
/// # Returns
///
/// `true` if the IP is in the range, `false` otherwise
///
/// # Security
///
/// - Validates prefix length is within valid range (0-32 for IPv4, 0-128 for IPv6)
/// - Returns `false` for invalid CIDR notation
fn ip_in_cidr(ip: &IpAddr, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let network_addr = match IpAddr::from_str(parts[0]) {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    let prefix_len: u8 = match parts[1].parse() {
        Ok(len) => len,
        Err(_) => return false,
    };

    match (ip, network_addr) {
        (IpAddr::V4(ip_v4), IpAddr::V4(net_v4)) => {
            // Validate prefix length for IPv4 (0-32)
            if prefix_len > 32 {
                warn!(cidr = %cidr, prefix_len = prefix_len, "Invalid IPv4 prefix length (must be 0-32)");
                return false;
            }

            let ip_u32 = u32::from(*ip_v4);
            let net_u32 = u32::from(net_v4);
            let mask = if prefix_len == 0 {
                0
            } else {
                !0u32 << (32 - prefix_len)
            };
            (ip_u32 & mask) == (net_u32 & mask)
        }
        (IpAddr::V6(ip_v6), IpAddr::V6(net_v6)) => {
            // Validate prefix length for IPv6 (0-128)
            if prefix_len > 128 {
                warn!(cidr = %cidr, prefix_len = prefix_len, "Invalid IPv6 prefix length (must be 0-128)");
                return false;
            }

            let ip_u128 = u128::from(*ip_v6);
            let net_u128 = u128::from(net_v6);
            let mask = if prefix_len == 0 {
                0
            } else {
                !0u128 << (128 - prefix_len)
            };
            (ip_u128 & mask) == (net_u128 & mask)
        }
        _ => false, // Mismatched IP versions
    }
}

/// Validate that a string is a valid IP address
///
/// # Arguments
///
/// * `ip` - The string to validate
///
/// # Returns
///
/// `true` if the string is a valid IPv4 or IPv6 address, `false` otherwise
fn is_valid_ip(ip: &str) -> bool {
    IpAddr::from_str(ip).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn test_extract_ip_from_peer() {
        let req = TestRequest::default()
            .peer_addr("192.168.1.100:8080".parse().unwrap())
            .to_http_request();

        let ip = extract_ip(&req);
        assert!(ip.starts_with("192.168.1.100"));
    }

    #[test]
    fn test_extract_ip_x_forwarded_for_right_to_left() {
        // SECURITY TEST: Right-to-left parsing prevents IP spoofing
        // Header: "spoofed-by-attacker, real-client, trusted-proxy"
        // Result: real-client (the first untrusted IP from right)
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .insert_header(("X-Forwarded-For", "198.51.100.99, 203.0.113.45, 10.0.0.1"))
            .to_http_request();

        let ip = extract_ip(&req);
        // Should return 203.0.113.45 (first untrusted IP from right)
        // NOT 198.51.100.99 (which could be spoofed by attacker)
        assert_eq!(ip, "203.0.113.45");
    }

    #[test]
    fn test_extract_ip_x_forwarded_for_single_ip() {
        // Single IP in chain (common case)
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .insert_header(("X-Forwarded-For", "203.0.113.45"))
            .to_http_request();

        let ip = extract_ip(&req);
        assert_eq!(ip, "203.0.113.45");
    }

    #[test]
    fn test_extract_ip_x_forwarded_for_all_trusted() {
        // All IPs in chain are trusted proxies - should fall through
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .insert_header(("X-Forwarded-For", "10.0.0.1, 192.168.1.1, 172.16.0.1"))
            .insert_header(("X-Real-IP", "203.0.113.45"))
            .to_http_request();

        let ip = extract_ip(&req);
        // Should fall through to X-Real-IP since all X-Forwarded-For are trusted
        assert_eq!(ip, "203.0.113.45");
    }

    #[test]
    fn test_extract_ip_x_real_ip() {
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .insert_header(("X-Real-IP", "203.0.113.45"))
            .to_http_request();

        let ip = extract_ip(&req);
        assert_eq!(ip, "203.0.113.45");
    }

    #[test]
    fn test_extract_ip_untrusted_proxy() {
        let req = TestRequest::default()
            .peer_addr("203.0.113.45:8080".parse().unwrap())
            .insert_header(("X-Forwarded-For", "198.51.100.1"))
            .to_http_request();

        // Should ignore X-Forwarded-For from untrusted IP
        let ip = extract_ip(&req);
        assert!(ip.starts_with("203.0.113.45"));
    }

    #[test]
    fn test_extract_ip_spoofing_attempt_blocked() {
        // Attacker tries to spoof their IP by adding fake entries to X-Forwarded-For
        // Real chain: client(attacker) -> proxy1 -> proxy2 -> our-server
        // Attacker sends: "fake-innocent-ip, attacker-ip" in X-Forwarded-For
        // Our proxy adds their actual IP: "fake-innocent-ip, attacker-ip, proxy1-ip"
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .insert_header((
                "X-Forwarded-For",
                "1.2.3.4, 5.6.7.8, 10.0.0.1, 192.168.1.1",
            ))
            .to_http_request();

        let ip = extract_ip(&req);
        // Should return 5.6.7.8 (first untrusted from right)
        // NOT 1.2.3.4 (which attacker tried to spoof)
        assert_eq!(ip, "5.6.7.8");
    }

    #[test]
    fn test_is_valid_ip() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("2001:db8::1"));
        assert!(!is_valid_ip("not-an-ip"));
        assert!(!is_valid_ip(""));
    }

    #[test]
    fn test_ip_in_cidr_ipv4() {
        let ip = IpAddr::from_str("10.1.2.3").unwrap();

        assert!(ip_in_cidr(&ip, "10.0.0.0/8"));
        assert!(ip_in_cidr(&ip, "10.1.2.0/24"));
        assert!(!ip_in_cidr(&ip, "192.168.0.0/16"));
    }

    #[test]
    fn test_ip_in_cidr_ipv4_invalid_prefix() {
        let ip = IpAddr::from_str("10.1.2.3").unwrap();

        // Invalid prefix length (> 32 for IPv4)
        assert!(!ip_in_cidr(&ip, "10.0.0.0/33"));
        assert!(!ip_in_cidr(&ip, "10.0.0.0/64"));
        assert!(!ip_in_cidr(&ip, "10.0.0.0/128"));
    }

    #[test]
    fn test_ip_in_cidr_ipv6() {
        let ip = IpAddr::from_str("2001:db8::1").unwrap();

        assert!(ip_in_cidr(&ip, "2001:db8::/32"));
        assert!(!ip_in_cidr(&ip, "2001:db9::/32"));
    }

    #[test]
    fn test_ip_in_cidr_ipv6_invalid_prefix() {
        let ip = IpAddr::from_str("2001:db8::1").unwrap();

        // Invalid prefix length (> 128 for IPv6)
        assert!(!ip_in_cidr(&ip, "2001:db8::/129"));
        assert!(!ip_in_cidr(&ip, "2001:db8::/200"));
    }

    #[test]
    fn test_ip_in_cidr_edge_cases() {
        let ip_v4 = IpAddr::from_str("10.1.2.3").unwrap();
        let ip_v6 = IpAddr::from_str("2001:db8::1").unwrap();

        // Prefix length 0 (matches all)
        assert!(ip_in_cidr(&ip_v4, "0.0.0.0/0"));
        assert!(ip_in_cidr(&ip_v6, "::/0"));

        // Maximum valid prefix length
        assert!(ip_in_cidr(&ip_v4, "10.1.2.3/32"));
        assert!(ip_in_cidr(&ip_v6, "2001:db8::1/128"));
    }

    #[test]
    fn test_is_trusted_proxy_localhost() {
        assert!(is_trusted_proxy("127.0.0.1"));
        assert!(is_trusted_proxy("::1"));
    }

    #[test]
    fn test_is_trusted_proxy_private_ranges() {
        assert!(is_trusted_proxy("10.1.2.3"));
        assert!(is_trusted_proxy("172.16.5.10"));
        assert!(is_trusted_proxy("192.168.1.1"));
    }

    #[test]
    fn test_is_trusted_proxy_public_ip() {
        assert!(!is_trusted_proxy("8.8.8.8"));
        assert!(!is_trusted_proxy("1.1.1.1"));
    }
}
