//! The network below HTTP.
//!
//! `http_request` covers most of what a program needs from a network, but not
//! all of it: checking whether a port is open before deploying to it, looking
//! up what a name resolves to, and speaking the small line-based protocols that
//! predate HTTP and are still what a mail server, a Redis, or a game server
//! answers on.
//!
//! Everything here is text, sent and received as UTF-8, and every call takes a
//! timeout in milliseconds - a network operation with no deadline is how a
//! program comes to hang forever with nothing in the log.

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A deadline, refusing the values that mean "wait forever" by accident.
fn deadline(timeout_milliseconds: i64, function_name: &str) -> Result<Duration, String> {
    if timeout_milliseconds < 1 {
        return Err(format!("{}: the timeout must be at least 1 millisecond, got {}", function_name, timeout_milliseconds));
    }
    return Ok(Duration::from_millis(timeout_milliseconds as u64));
}

/// Opens a connection, sends the text, and returns everything sent back until
/// the other end closes or the timeout runs out. For the request-then-response
/// protocols this suits, that is the whole conversation.
///
/// The text is sent exactly as given - if the protocol wants a line ending, put
/// one in - because which line ending is part of the protocol, not of this.
pub async fn tcp_request(host: String, port: i64, text: String, timeout_milliseconds: i64) -> Result<String, String> {
    let wait = deadline(timeout_milliseconds, "net_tcp_request")?;
    if !(1..=65535).contains(&port) {
        return Err(format!("net_tcp_request: {} is not a port", port));
    }
    let address = format!("{}:{}", host, port);

    let exchange = async {
        let mut stream = tokio::net::TcpStream::connect(&address).await.map_err(|failure| format!("net_tcp_request: could not connect to {}: {}", address, failure))?;
        stream.write_all(text.as_bytes()).await.map_err(|failure| format!("net_tcp_request: could not send to {}: {}", address, failure))?;
        // Some protocols only answer once the sending side is done, and the
        // rest do not mind being told early that nothing more is coming.
        let _ = stream.shutdown().await;
        let mut received = Vec::new();
        stream.read_to_end(&mut received).await.map_err(|failure| format!("net_tcp_request: could not read from {}: {}", address, failure))?;
        return Ok::<Vec<u8>, String>(received);
    };

    let received = match tokio::time::timeout(wait, exchange).await {
        Ok(result) => result?,
        Err(_) => return Err(format!("net_tcp_request: {} did not finish answering within {}ms", address, timeout_milliseconds)),
    };
    return String::from_utf8(received).map_err(|_| format!("net_tcp_request: {} sent back something that is not text", address));
}

/// Whether something is listening on a port. This is what a deploy script asks
/// before it tries to talk to the thing it just started, and what a health
/// check asks about a service on another machine.
///
/// A port that refuses the connection and a host that cannot be reached are
/// both simply false - the question is whether it is open, and the reason it is
/// not belongs to whoever is debugging it.
pub async fn tcp_is_open(host: String, port: i64, timeout_milliseconds: i64) -> Result<bool, String> {
    let wait = deadline(timeout_milliseconds, "net_tcp_is_open")?;
    if !(1..=65535).contains(&port) {
        return Err(format!("net_tcp_is_open: {} is not a port", port));
    }
    let address = format!("{}:{}", host, port);
    return match tokio::time::timeout(wait, tokio::net::TcpStream::connect(&address)).await {
        Ok(Ok(_)) => Ok(true),
        Ok(Err(_)) => Ok(false),
        Err(_) => Ok(false),
    };
}

/// Sends one datagram and waits for one back - the shape of a UDP protocol like
/// DNS, NTP or a game's status query. UDP does not guarantee either datagram
/// arrives, so a timeout here means "no answer came", not "the host is down".
pub async fn udp_request(host: String, port: i64, text: String, timeout_milliseconds: i64) -> Result<String, String> {
    let wait = deadline(timeout_milliseconds, "net_udp_request")?;
    if !(1..=65535).contains(&port) {
        return Err(format!("net_udp_request: {} is not a port", port));
    }
    let address = format!("{}:{}", host, port);

    let exchange = async {
        // Port 0 asks the operating system for any free local port, which is
        // what a client wants.
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.map_err(|failure| format!("net_udp_request: could not open a local socket: {}", failure))?;
        socket.send_to(text.as_bytes(), &address).await.map_err(|failure| format!("net_udp_request: could not send to {}: {}", address, failure))?;
        let mut buffer = vec![0u8; 65535];
        let (received, _from) = socket.recv_from(&mut buffer).await.map_err(|failure| format!("net_udp_request: could not read from {}: {}", address, failure))?;
        buffer.truncate(received);
        return Ok::<Vec<u8>, String>(buffer);
    };

    let received = match tokio::time::timeout(wait, exchange).await {
        Ok(result) => result?,
        Err(_) => return Err(format!("net_udp_request: no answer came from {} within {}ms", address, timeout_milliseconds)),
    };
    return String::from_utf8(received).map_err(|_| format!("net_udp_request: {} sent back something that is not text", address));
}

/// Every address a hostname resolves to, as text. Both IPv4 and IPv6 addresses
/// come back, in the order the resolver gave them, since that order is the
/// resolver's advice about which to try first.
pub async fn dns_lookup(hostname: String) -> Result<Vec<String>, String> {
    // `lookup_host` wants something with a port, and the port is then discarded.
    let addresses = tokio::net::lookup_host(format!("{}:0", hostname)).await.map_err(|failure| format!("net_dns_lookup: could not look up '{}': {}", hostname, failure))?;
    let found: Vec<String> = addresses.map(|address| address.ip().to_string()).collect();
    if found.is_empty() {
        return Err(format!("net_dns_lookup: '{}' resolved to no addresses", hostname));
    }
    return Ok(found);
}

/// A resolver reading the machine's own DNS settings, so a lookup goes to the
/// same servers everything else on the box uses. Built per call: these
/// functions are asked once at a time, and a resolver held for the life of the
/// program would be a piece of global state this module does not otherwise
/// have.
#[cfg(feature = "dns")]
fn resolver() -> hickory_resolver::TokioAsyncResolver {
    return match hickory_resolver::TokioAsyncResolver::tokio_from_system_conf() {
        Ok(resolver) => resolver,
        // A machine with no resolver configuration of its own still has the
        // public servers, which is better than refusing to look anything up.
        Err(_) => {
            let mut options = hickory_resolver::config::ResolverOpts::default();
            options.timeout = Duration::from_secs(5);
            hickory_resolver::TokioAsyncResolver::tokio(hickory_resolver::config::ResolverConfig::default(), options)
        }
    };
}

/// The mail hosts a domain names, in the order mail should be tried: the lowest
/// preference number first, which is what the numbers mean. This is the lookup
/// behind "does this address have anywhere to deliver to", the check worth
/// doing on a typed email address that `validate_email` cannot make.
///
/// A domain with no mail hosts is an error rather than an empty list, because
/// that is the answer that means something: nothing accepts mail there.
#[cfg(feature = "dns")]
pub async fn dns_mx(domain: String) -> Result<Vec<String>, String> {
    let lookup = resolver().mx_lookup(domain.clone()).await.map_err(|failure| format!("net_dns_mx: could not look up the mail hosts for '{}': {}", domain, failure))?;

    let mut hosts: Vec<(u16, String)> = lookup.iter().map(|record| (record.preference(), record.exchange().to_utf8().trim_end_matches('.').to_string())).collect();
    hosts.sort_by(|first, second| first.0.cmp(&second.0).then_with(|| first.1.cmp(&second.1)));
    if hosts.is_empty() {
        return Err(format!("net_dns_mx: '{}' names no mail hosts", domain));
    }
    return Ok(hosts.into_iter().map(|(_, host)| host).collect());
}

/// The text records on a name, one string per record. This is where a domain
/// keeps the things other machines need to be told about it: an SPF or DMARC
/// policy, a verification token a service asked to have put there, a public key
/// for signing mail.
///
/// A record split into several quoted pieces comes back joined, which is how
/// every reader of these is supposed to treat it.
#[cfg(feature = "dns")]
pub async fn dns_txt(name: String) -> Result<Vec<String>, String> {
    let lookup = resolver().txt_lookup(name.clone()).await.map_err(|failure| format!("net_dns_txt: could not look up the text records for '{}': {}", name, failure))?;

    let records: Vec<String> = lookup
        .iter()
        .map(|record| record.txt_data().iter().map(|piece| String::from_utf8_lossy(piece).to_string()).collect::<Vec<String>>().join(""))
        .collect();
    if records.is_empty() {
        return Err(format!("net_dns_txt: '{}' has no text records", name));
    }
    return Ok(records);
}

/// The names an address points back at. What a log line showing an address can
/// be turned into for a person to read, and the first half of the forward-
/// confirmed check that tells a crawler apart from something claiming to be one.
///
/// Most addresses have no reverse name at all, and that is an error rather than
/// an empty list.
#[cfg(feature = "dns")]
pub async fn dns_reverse(ip: String) -> Result<Vec<String>, String> {
    let address: std::net::IpAddr = ip.parse().map_err(|_| format!("net_dns_reverse: '{}' is not an IP address", ip))?;
    let lookup = resolver().reverse_lookup(address).await.map_err(|failure| format!("net_dns_reverse: could not look up the name for '{}': {}", ip, failure))?;

    let names: Vec<String> = lookup.iter().map(|name| name.to_utf8().trim_end_matches('.').to_string()).collect();
    if names.is_empty() {
        return Err(format!("net_dns_reverse: '{}' has no reverse name", ip));
    }
    return Ok(names);
}

/// When the certificate a server presents stops being valid, as a Unix
/// timestamp to compare with `time_now`.
///
/// This is the number behind every "the certificate expired on Saturday"
/// outage: a program that checks it on a schedule knows weeks ahead. The
/// certificate is read from a real TLS handshake, so what comes back is what
/// the server is actually serving today, not what a renewal script believes it
/// installed.
///
/// A certificate that is already expired, or that no trusted authority signed,
/// fails the handshake and the error says which - so the alert fires either way.
#[cfg(feature = "tls")]
pub async fn tls_cert_expiry(hostname: String, port: i64, timeout_milliseconds: i64) -> Result<i64, String> {
    use x509_parser::prelude::FromDer;

    let wait = deadline(timeout_milliseconds, "net_tls_cert_expiry")?;
    if !(1..=65535).contains(&port) {
        return Err(format!("net_tls_cert_expiry: {} is not a port", port));
    }

    let mut roots = tokio_rustls::rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let settings = tokio_rustls::rustls::ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(settings));
    let server_name = tokio_rustls::rustls::pki_types::ServerName::try_from(hostname.clone())
        .map_err(|_| format!("net_tls_cert_expiry: '{}' is not a hostname a certificate can be checked against", hostname))?;

    let handshake = async {
        let stream = tokio::net::TcpStream::connect(format!("{}:{}", hostname, port))
            .await
            .map_err(|failure| format!("net_tls_cert_expiry: could not connect to {}:{}: {}", hostname, port, failure))?;
        return connector.connect(server_name, stream).await.map_err(|failure| format!("net_tls_cert_expiry: {}:{} did not present a usable certificate: {}", hostname, port, failure));
    };
    let connection = match tokio::time::timeout(wait, handshake).await {
        Ok(result) => result?,
        Err(_) => return Err(format!("net_tls_cert_expiry: {}:{} did not answer within {}ms", hostname, port, timeout_milliseconds)),
    };

    let (_, session) = connection.get_ref();
    let presented = session.peer_certificates().ok_or_else(|| format!("net_tls_cert_expiry: {}:{} presented no certificate", hostname, port))?;
    // The server's own certificate is the first one - the rest are the chain up
    // to an authority, and they expire on their own schedules.
    let own = presented.first().ok_or_else(|| format!("net_tls_cert_expiry: {}:{} presented no certificate", hostname, port))?;
    let (_, parsed) = x509_parser::certificate::X509Certificate::from_der(own.as_ref())
        .map_err(|failure| format!("net_tls_cert_expiry: {}:{} presented a certificate this cannot read: {}", hostname, port, failure))?;
    return Ok(parsed.validity().not_after.timestamp());
}

/// How many whole days are left before the certificate a server presents stops
/// being valid - the number an alert compares against. Fourteen days left is a
/// renewal that has not happened yet.
///
/// Part of a day does not count, so this is the number of days a program can
/// still wait, not the number it has been running for.
#[cfg(feature = "tls")]
pub async fn tls_cert_days_left(hostname: String, port: i64, timeout_milliseconds: i64) -> Result<i64, String> {
    let expires_at = tls_cert_expiry(hostname, port, timeout_milliseconds).await.map_err(|failure| failure.replace("net_tls_cert_expiry:", "net_tls_cert_days_left:"))?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.as_secs() as i64).unwrap_or(0);
    return Ok((expires_at - now).div_euclid(86_400));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_name_resolves_to_at_least_one_address() {
        let addresses = dns_lookup("localhost".to_string()).await.expect("localhost always resolves");
        assert!(!addresses.is_empty());
        assert!(addresses.iter().any(|address| address == "127.0.0.1" || address == "::1"), "got: {:?}", addresses);
    }

    #[tokio::test]
    async fn a_name_that_does_not_resolve_says_so() {
        let failure = dns_lookup("this-name-does-not-exist.invalid".to_string()).await.unwrap_err();
        assert!(failure.contains("net_dns_lookup"), "got: {}", failure);
    }

    #[tokio::test]
    async fn a_port_with_something_listening_is_open_and_one_without_is_not() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let port = listener.local_addr().expect("a bound address").port() as i64;
        assert!(tcp_is_open("127.0.0.1".to_string(), port, 1000).await.expect("a valid port"));

        drop(listener);
        // Nothing is listening now, so the connection is refused - which is an
        // answer of false rather than an error.
        assert!(!tcp_is_open("127.0.0.1".to_string(), port, 1000).await.expect("a valid port"));
    }

    #[tokio::test]
    async fn a_request_gets_back_what_the_other_end_sent() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let port = listener.local_addr().expect("a bound address").port() as i64;
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("a connection");
            let mut received = Vec::new();
            let _ = stream.read_to_end(&mut received).await;
            let answer = format!("heard: {}", String::from_utf8_lossy(&received));
            let _ = stream.write_all(answer.as_bytes()).await;
        });

        let answer = tcp_request("127.0.0.1".to_string(), port, "PING\r\n".to_string(), 2000).await.expect("a listening server");
        assert_eq!(answer, "heard: PING\r\n");
    }

    #[tokio::test]
    async fn a_datagram_gets_one_back() {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.expect("a free port");
        let port = server.local_addr().expect("a bound address").port() as i64;
        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            let (received, from) = server.recv_from(&mut buffer).await.expect("a datagram");
            let answer = format!("heard: {}", String::from_utf8_lossy(&buffer[..received]));
            let _ = server.send_to(answer.as_bytes(), from).await;
        });

        let answer = udp_request("127.0.0.1".to_string(), port, "STATUS".to_string(), 2000).await.expect("a listening server");
        assert_eq!(answer, "heard: STATUS");
    }

    #[tokio::test]
    async fn nothing_waits_forever() {
        assert!(tcp_request("127.0.0.1".to_string(), 9, "x".to_string(), 0).await.is_err());
        assert!(tcp_is_open("127.0.0.1".to_string(), 9, -1).await.is_err());
        assert!(udp_request("127.0.0.1".to_string(), 9, "x".to_string(), 0).await.is_err());
    }

    #[tokio::test]
    async fn something_that_is_not_a_port_is_refused() {
        assert!(tcp_request("127.0.0.1".to_string(), 0, "x".to_string(), 100).await.is_err());
        assert!(tcp_is_open("127.0.0.1".to_string(), 70000, 100).await.is_err());
        assert!(udp_request("127.0.0.1".to_string(), -5, "x".to_string(), 100).await.is_err());
    }

    /// A datagram that never gets an answer is a timeout, not a hang.
    #[tokio::test]
    async fn a_udp_request_nobody_answers_times_out() {
        let failure = udp_request("127.0.0.1".to_string(), 9, "x".to_string(), 200).await.unwrap_err();
        assert!(failure.contains("no answer came"), "got: {}", failure);
    }
}

fn parse_ip(text: &str, what: &str) -> Result<std::net::IpAddr, String> {
    return text.trim().parse::<std::net::IpAddr>().map_err(|_| format!("{}: `{}` is not an IP address", what, text.trim()));
}

/// Whether an address sits inside a CIDR range like `10.0.0.0/8` or `fd00::/8`.
/// An address of the other family is simply outside the range, not an error.
pub fn ip_in_cidr(ip: String, cidr: String) -> Result<bool, String> {
    let address = parse_ip(&ip, "net_ip_in_cidr")?;
    let trimmed = cidr.trim();
    let (base_text, prefix_text) = trimmed.split_once('/').ok_or_else(|| format!("net_ip_in_cidr: `{}` is not CIDR notation - it needs a /prefix", trimmed))?;
    let base = parse_ip(base_text, "net_ip_in_cidr")?;
    let prefix: u32 = prefix_text.trim().parse().map_err(|_| format!("net_ip_in_cidr: `{}` is not a prefix length", prefix_text))?;
    return match (address, base) {
        (std::net::IpAddr::V4(a), std::net::IpAddr::V4(b)) => {
            if prefix > 32 {
                return Err(format!("net_ip_in_cidr: a v4 prefix runs 0 to 32, not {}", prefix));
            }
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            Ok(u32::from(a) & mask == u32::from(b) & mask)
        }
        (std::net::IpAddr::V6(a), std::net::IpAddr::V6(b)) => {
            if prefix > 128 {
                return Err(format!("net_ip_in_cidr: a v6 prefix runs 0 to 128, not {}", prefix));
            }
            let mask = if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
            Ok(u128::from(a) & mask == u128::from(b) & mask)
        }
        _ => Ok(false),
    };
}

/// Whether an address is private - RFC 1918 space for v4, unique-local for v6.
pub fn ip_is_private(ip: String) -> Result<bool, String> {
    return match parse_ip(&ip, "net_ip_is_private")? {
        std::net::IpAddr::V4(v4) => Ok(v4.is_private()),
        std::net::IpAddr::V6(v6) => Ok((v6.segments()[0] & 0xfe00) == 0xfc00),
    };
}

/// Whether an address points back at the machine itself.
pub fn ip_is_loopback(ip: String) -> Result<bool, String> {
    return Ok(parse_ip(&ip, "net_ip_is_loopback")?.is_loopback());
}

/// 4 or 6, after checking the address really is one.
pub fn ip_version(ip: String) -> Result<i64, String> {
    return match parse_ip(&ip, "net_ip_version")? {
        std::net::IpAddr::V4(_) => Ok(4),
        std::net::IpAddr::V6(_) => Ok(6),
    };
}

/// A v4 address as the integer it is - what log databases store and range
/// comparisons sort. A v6 address does not fit and says so.
pub fn ip_to_int(ip: String) -> Result<i64, String> {
    return match parse_ip(&ip, "net_ip_to_int")? {
        std::net::IpAddr::V4(v4) => Ok(u32::from(v4) as i64),
        std::net::IpAddr::V6(_) => Err("net_ip_to_int: only a v4 address fits in an integer".to_string()),
    };
}

/// The integer back to its dotted v4 form.
pub fn ip_from_int(value: i64) -> Result<String, String> {
    if value < 0 || value > u32::MAX as i64 {
        return Err(format!("net_ip_from_int: {} is outside the v4 address space", value));
    }
    return Ok(std::net::Ipv4Addr::from(value as u32).to_string());
}

#[cfg(test)]
mod ip_tests {
    use super::*;

    #[test]
    fn cidr_membership_reads_correctly() {
        assert!(ip_in_cidr("10.1.2.3".to_string(), "10.0.0.0/8".to_string()).unwrap());
        assert!(!ip_in_cidr("11.1.2.3".to_string(), "10.0.0.0/8".to_string()).unwrap());
        assert!(ip_in_cidr("192.168.1.7".to_string(), "192.168.1.0/24".to_string()).unwrap());
        assert!(ip_in_cidr("fd12::1".to_string(), "fd00::/8".to_string()).unwrap());
        assert!(!ip_in_cidr("10.0.0.1".to_string(), "fd00::/8".to_string()).unwrap());
        assert!(ip_in_cidr("8.8.8.8".to_string(), "0.0.0.0/0".to_string()).unwrap());
        assert!(ip_in_cidr("10.9.9.9".to_string(), "10.128.0.0/8".to_string()).unwrap(), "the base's host bits are masked off");
    }

    #[test]
    fn bad_cidr_text_is_refused() {
        assert!(ip_in_cidr("10.0.0.1".to_string(), "10.0.0.0".to_string()).unwrap_err().contains("needs a /prefix"));
        assert!(ip_in_cidr("10.0.0.1".to_string(), "10.0.0.0/33".to_string()).unwrap_err().contains("0 to 32"));
        assert!(ip_in_cidr("not-an-ip".to_string(), "10.0.0.0/8".to_string()).unwrap_err().contains("not an IP address"));
    }

    #[test]
    fn the_private_and_loopback_families_are_known() {
        assert!(ip_is_private("192.168.0.1".to_string()).unwrap());
        assert!(ip_is_private("10.0.0.1".to_string()).unwrap());
        assert!(ip_is_private("fd00::1".to_string()).unwrap());
        assert!(!ip_is_private("8.8.8.8".to_string()).unwrap());
        assert!(ip_is_loopback("127.0.0.1".to_string()).unwrap());
        assert!(ip_is_loopback("::1".to_string()).unwrap());
        assert_eq!(ip_version("8.8.8.8".to_string()).unwrap(), 4);
        assert_eq!(ip_version("::1".to_string()).unwrap(), 6);
    }

    #[test]
    fn v4_addresses_round_trip_through_integers() {
        let as_int = ip_to_int("1.2.3.4".to_string()).unwrap();
        assert_eq!(as_int, 16909060);
        assert_eq!(ip_from_int(as_int).unwrap(), "1.2.3.4");
        assert!(ip_to_int("::1".to_string()).is_err());
        assert!(ip_from_int(-1).is_err());
    }
}

pub type LineFuture = std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>>;

fn check_port(port: i64, what: &str) -> Result<u16, String> {
    if !(1..=65535).contains(&port) {
        return Err(format!("{}: a port runs 1 to 65535, not {}", what, port));
    }
    return Ok(port as u16);
}

/// Accept TCP connections and speak a line-at-a-time protocol: each line a
/// client sends is answered by the program's handle_line function, and an
/// empty reply sends nothing back. Blocks forever, so it runs in a spawn
/// block. Bind `127.0.0.1` to stay behind a reverse proxy, `0.0.0.0` to face
/// the world.
pub async fn tcp_serve<F>(host: String, port: i64, handler: F) -> Result<(), String>
where
    F: Fn(String) -> LineFuture + Clone + Send + Sync + 'static,
{
    let port = check_port(port, "net_tcp_serve")?;
    let listener = tokio::net::TcpListener::bind((host.trim(), port))
        .await
        .map_err(|e| format!("net_tcp_serve: could not listen on {}:{}: {}", host.trim(), port, e))?;
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(_) => continue,
        };
        let handler = handler.clone();
        tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
            let (read_half, mut write_half) = stream.into_split();
            let mut lines = tokio::io::BufReader::new(read_half).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let reply = handler(line).await;
                if !reply.is_empty() {
                    if write_half.write_all(reply.as_bytes()).await.is_err() {
                        break;
                    }
                    if write_half.write_all(b"\n").await.is_err() {
                        break;
                    }
                }
            }
        });
    }
}

/// Answer UDP datagrams: each one that arrives is passed as text to the
/// program's handle_packet function, and a non-empty reply is sent back to
/// whoever asked. Blocks forever, so it runs in a spawn block.
pub async fn udp_serve<F>(host: String, port: i64, handler: F) -> Result<(), String>
where
    F: Fn(String) -> LineFuture + Clone + Send + Sync + 'static,
{
    let port = check_port(port, "net_udp_serve")?;
    let socket = tokio::net::UdpSocket::bind((host.trim(), port))
        .await
        .map_err(|e| format!("net_udp_serve: could not listen on {}:{}: {}", host.trim(), port, e))?;
    let mut buffer = vec![0u8; 65536];
    loop {
        let (size, source) = match socket.recv_from(&mut buffer).await {
            Ok(received) => received,
            Err(_) => continue,
        };
        let text = String::from_utf8_lossy(&buffer[..size]).to_string();
        let reply = handler(text).await;
        if !reply.is_empty() {
            let _ = socket.send_to(reply.as_bytes(), source).await;
        }
    }
}

#[cfg(test)]
mod serve_tests {
    use super::*;

    #[tokio::test]
    async fn a_tcp_server_answers_line_for_line() {
        let port = 41893;
        tokio::spawn(async move {
            let _ = tcp_serve("127.0.0.1".to_string(), port, |line| {
                Box::pin(async move { format!("echo {}", line) }) as LineFuture
            })
            .await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let answer = tcp_request("127.0.0.1".to_string(), port, "hello\n".to_string(), 1000).await.expect("an answer");
        assert!(answer.starts_with("echo hello"), "got: {}", answer);
    }

    #[tokio::test]
    async fn a_udp_server_answers_datagrams() {
        let port = 41894;
        tokio::spawn(async move {
            let _ = udp_serve("127.0.0.1".to_string(), port, |text| {
                Box::pin(async move { text.to_uppercase() }) as LineFuture
            })
            .await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let answer = udp_request("127.0.0.1".to_string(), port, "ping".to_string(), 1000).await.expect("an answer");
        assert_eq!(answer, "PING");
    }

    #[tokio::test]
    async fn a_bad_port_is_refused() {
        let failure = tcp_serve("127.0.0.1".to_string(), 700000, |_| Box::pin(async { String::new() }) as LineFuture).await.unwrap_err();
        assert!(failure.contains("1 to 65535"));
    }
}

/// These reach the real network, so they are written to say something useful
/// whichever way they land: a machine with no way out gets the error path
/// checked, a machine with one gets both.
#[cfg(all(test, feature = "dns"))]
mod name_lookup_tests {
    use super::*;

    #[tokio::test]
    async fn a_domain_with_mail_names_its_hosts_in_preference_order() {
        match dns_mx("gmail.com".to_string()).await {
            Ok(hosts) => {
                assert!(!hosts.is_empty());
                assert!(hosts.iter().all(|host| !host.ends_with('.')), "the trailing dot is not part of a hostname: {:?}", hosts);
                assert!(hosts[0].contains("google"), "the first host is the one to try first: {:?}", hosts);
            }
            Err(failure) => assert!(failure.contains("net_dns_mx"), "got: {}", failure),
        }
    }

    #[tokio::test]
    async fn a_name_with_no_mail_and_no_records_says_so() {
        let failure = dns_mx("this-domain-does-not-exist.invalid".to_string()).await.unwrap_err();
        assert!(failure.contains("net_dns_mx"), "got: {}", failure);
        let failure = dns_txt("this-domain-does-not-exist.invalid".to_string()).await.unwrap_err();
        assert!(failure.contains("net_dns_txt"), "got: {}", failure);
    }

    #[tokio::test]
    async fn text_records_come_back_whole() {
        match dns_txt("gmail.com".to_string()).await {
            Ok(records) => assert!(records.iter().any(|record| record.contains("spf")), "a domain that sends mail publishes an SPF policy: {:?}", records),
            Err(failure) => assert!(failure.contains("net_dns_txt"), "got: {}", failure),
        }
    }

    #[tokio::test]
    async fn a_reverse_lookup_needs_an_address_to_start_from() {
        let failure = dns_reverse("not-an-address".to_string()).await.unwrap_err();
        assert!(failure.contains("is not an IP address"), "got: {}", failure);

        match dns_reverse("8.8.8.8".to_string()).await {
            Ok(names) => assert!(names.iter().any(|name| name.contains("dns.google")), "got: {:?}", names),
            Err(failure) => assert!(failure.contains("net_dns_reverse"), "got: {}", failure),
        }
    }
}

#[cfg(all(test, feature = "tls"))]
mod certificate_tests {
    use super::*;

    #[tokio::test]
    async fn a_certificate_says_when_it_stops_being_valid() {
        match tls_cert_expiry("example.com".to_string(), 443, 8000).await {
            Ok(expires_at) => {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("a clock after 1970").as_secs() as i64;
                assert!(expires_at > now, "a certificate in use has not expired yet");
                // No public authority issues for longer than about 400 days.
                assert!(expires_at - now < 400 * 86_400, "got {} seconds of validity left", expires_at - now);

                let days = tls_cert_days_left("example.com".to_string(), 443, 8000).await.expect("the same handshake twice");
                assert!(days >= 0 && days <= 400);
            }
            Err(failure) => assert!(failure.contains("net_tls_cert_expiry"), "got: {}", failure),
        }
    }

    #[tokio::test]
    async fn a_port_that_is_not_a_port_and_a_timeout_that_is_not_one_are_refused() {
        assert!(tls_cert_expiry("example.com".to_string(), 0, 1000).await.unwrap_err().contains("is not a port"));
        assert!(tls_cert_expiry("example.com".to_string(), 443, 0).await.unwrap_err().contains("at least 1 millisecond"));
    }

    /// Nothing is listening, so this is the connection-refused path rather than
    /// a handshake, and it must still name the function that failed.
    #[tokio::test]
    async fn a_server_that_is_not_there_says_so() {
        let failure = tls_cert_days_left("127.0.0.1".to_string(), 1, 1000).await.unwrap_err();
        assert!(failure.contains("net_tls_cert_days_left"), "got: {}", failure);
    }
}
