//! Net module stdlib registry entries.
//!
//! Every call takes a timeout in milliseconds, because a network operation with
//! no deadline is how a program hangs forever with nothing in the log.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Net:
        "net_tcp_request" [Tokio] => "std_lib::net::tcp_request", (host: s, port: i, text: s, timeout_milliseconds: i) -> (s!e),
            "Opens a TCP connection, sends the text exactly as given, and returns everything sent back until the other end closes or the timeout runs out.",
            "greeting:s = danger(net_tcp_request(`localhost`, 25, `EHLO nail\\r\\n`, 2000));";
        "net_tcp_is_open" [Tokio] => "std_lib::net::tcp_is_open", (host: s, port: i, timeout_milliseconds: i) -> (b!e),
            "Returns whether something is listening on a port. A refused connection and an unreachable host are both false.",
            "ready:b = danger(net_tcp_is_open(`127.0.0.1`, 8080, 1000));";
        "net_udp_request" [Tokio] => "std_lib::net::udp_request", (host: s, port: i, text: s, timeout_milliseconds: i) -> (s!e),
            "Sends one UDP datagram and waits for one back. A timeout means no answer came, not that the host is down.",
            "status:s = danger(net_udp_request(`127.0.0.1`, 27015, `STATUS`, 1000));";
        "net_dns_lookup" [Tokio] => "std_lib::net::dns_lookup", (hostname: s) -> ([s]!e),
            "Returns every address a hostname resolves to, in the order the resolver gave them.",
            "addresses:a:s = danger(net_dns_lookup(`nail-lang.org`));";
    }
}
