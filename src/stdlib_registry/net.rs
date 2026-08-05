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
        "net_ip_in_cidr" => "std_lib::net::ip_in_cidr", (ip: s, cidr: s) -> (b!e),
            "Whether an address sits inside a CIDR range like `10.0.0.0/8` - how an allowlist is checked. An address of the other family is outside the range, not an error.",
            "allowed:b = danger(net_ip_in_cidr(client_ip, `10.0.0.0/8`));";
        "net_ip_is_private" => "std_lib::net::ip_is_private", (ip: s) -> (b!e),
            "Whether an address is private - RFC 1918 space for v4, unique-local for v6.",
            "internal:b = danger(net_ip_is_private(client_ip));";
        "net_ip_is_loopback" => "std_lib::net::ip_is_loopback", (ip: s) -> (b!e),
            "Whether an address points back at the machine itself.",
            "local:b = danger(net_ip_is_loopback(client_ip));";
        "net_ip_version" => "std_lib::net::ip_version", (ip: s) -> (i!e),
            "4 or 6, after checking the text really is an address.",
            "version:i = danger(net_ip_version(client_ip));";
        "net_ip_to_int" => "std_lib::net::ip_to_int", (ip: s) -> (i!e),
            "A v4 address as the integer it is - what log databases store and range comparisons sort. A v6 address does not fit and says so.",
            "key:i = danger(net_ip_to_int(client_ip));";
        "net_ip_from_int" => "std_lib::net::ip_from_int", (value: i) -> (s!e),
            "The integer back to its dotted v4 form.",
            "address:s = danger(net_ip_from_int(16909060));";
        "net_tcp_serve" [Tokio] => "std_lib::net::tcp_serve", (host: s, port: i) -> (v!e),
            "Accepts TCP connections and speaks a line-at-a-time protocol: each line a client sends is answered by the program's handle_line(line:s):s function, and an empty reply sends nothing back. Blocks forever, so it runs in a spawn block. Bind `127.0.0.1` to stay behind a reverse proxy, `0.0.0.0` to face the world.",
            "spawn { danger(net_tcp_serve(`127.0.0.1`, 4000)); }";
        "net_udp_serve" [Tokio] => "std_lib::net::udp_serve", (host: s, port: i) -> (v!e),
            "Answers UDP datagrams: each one is passed as text to the program's handle_packet(packet:s):s function, and a non-empty reply goes back to whoever asked. Blocks forever, so it runs in a spawn block.",
            "spawn { danger(net_udp_serve(`0.0.0.0`, 27015)); }";
    }
}
