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
