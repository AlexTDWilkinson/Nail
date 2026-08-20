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
        "net_dns_mx" [HickoryResolver, Tokio] => "std_lib::net::dns_mx", (domain: s) -> ([s]!e),
            "Returns the mail hosts a domain names, in the order mail should be tried. This is the check worth doing on a typed email address that validate_email cannot make: whether anything accepts mail there at all. A domain naming none is an error.",
            "hosts:a:s = danger(net_dns_mx(`example.com`));";
        "net_dns_txt" [HickoryResolver, Tokio] => "std_lib::net::dns_txt", (name: s) -> ([s]!e),
            "Returns the text records on a name, one string per record - an SPF or DMARC policy, a verification token a service asked to have put there, a signing key. A record written in several quoted pieces comes back joined.",
            "records:a:s = danger(net_dns_txt(`example.com`));";
        "net_dns_reverse" [HickoryResolver, Tokio] => "std_lib::net::dns_reverse", (ip: s) -> ([s]!e),
            "Returns the names an address points back at, for turning a log line into something a person reads. Most addresses have no reverse name, and that is an error rather than an empty list.",
            "names:a:s = danger(net_dns_reverse(`8.8.8.8`));";
        "net_tls_cert_expiry" [TokioRustls, WebpkiRoots, X509Parser, Tokio] => "std_lib::net::tls_cert_expiry", (hostname: s, port: i, timeout_milliseconds: i) -> (i!e),
            "Returns when the certificate a server presents stops being valid, as a Unix timestamp to compare with time_now. Read from a real handshake, so it is what the server serves today rather than what a renewal script believes it installed.",
            "expires:i = danger(net_tls_cert_expiry(`nail-lang.org`, 443, 5000));";
        "net_tls_cert_days_left" [TokioRustls, WebpkiRoots, X509Parser, Tokio] => "std_lib::net::tls_cert_days_left", (hostname: s, port: i, timeout_milliseconds: i) -> (i!e),
            "Returns how many whole days are left before the certificate a server presents stops being valid - the number a scheduled check compares against. An expired or untrusted certificate fails the handshake and the error says which.",
            "days:i = danger(net_tls_cert_days_left(`nail-lang.org`, 443, 5000));";
        "net_ip_in_cidr" => "std_lib::net::ip_in_cidr", (ip: s, cidr: s) -> (b!e),
            "Whether an address sits inside a CIDR range like `10.0.0.0/8` - how an allowlist is checked. An address of the other family is outside the range, not an error.",
            "client_ip:s = `10.1.2.3`;\nallowed:b = danger(net_ip_in_cidr(client_ip, `10.0.0.0/8`));";
        "net_ip_is_private" => "std_lib::net::ip_is_private", (ip: s) -> (b!e),
            "Whether an address is private - RFC 1918 space for v4, unique-local for v6.",
            "client_ip:s = `10.1.2.3`;\ninternal:b = danger(net_ip_is_private(client_ip));";
        "net_ip_is_loopback" => "std_lib::net::ip_is_loopback", (ip: s) -> (b!e),
            "Whether an address points back at the machine itself.",
            "client_ip:s = `10.1.2.3`;\nlocal:b = danger(net_ip_is_loopback(client_ip));";
        "net_ip_version" => "std_lib::net::ip_version", (ip: s) -> (i!e),
            "4 or 6, after checking the text really is an address.",
            "client_ip:s = `10.1.2.3`;\nversion:i = danger(net_ip_version(client_ip));";
        "net_ip_to_int" => "std_lib::net::ip_to_int", (ip: s) -> (i!e),
            "A v4 address as the integer it is - what log databases store and range comparisons sort. A v6 address does not fit and says so.",
            "client_ip:s = `10.1.2.3`;\nkey:i = danger(net_ip_to_int(client_ip));";
        "net_ip_from_int" => "std_lib::net::ip_from_int", (value: i) -> (s!e),
            "The integer back to its dotted v4 form.",
            "address:s = danger(net_ip_from_int(16909060));";
        "net_tcp_serve" [Tokio] => "std_lib::net::tcp_serve", (host: s, port: i) -> (v!e),
            "Accepts TCP connections and speaks a line-at-a-time protocol: each line a client sends is answered by the program's handle_line(line:s):s function, and an empty reply sends nothing back. Blocks forever, so it runs in a c block beside the rest of the program. Bind `127.0.0.1` to stay behind a reverse proxy, `0.0.0.0` to face the world.",
            "f handle_line(line:s):s {\n    r string_concat([`you said: `, line]);\n}\n\ndanger(net_tcp_serve(`127.0.0.1`, 4000));";
        "net_udp_serve" [Tokio] => "std_lib::net::udp_serve", (host: s, port: i) -> (v!e),
            "Answers UDP datagrams: each one is passed as text to the program's handle_packet(packet:s):s function, and a non-empty reply goes back to whoever asked. Blocks forever, so it runs in a c block beside the rest of the program.",
            "f handle_packet(text:s):s {\n    r string_concat([`heard: `, text]);\n}\n\ndanger(net_udp_serve(`0.0.0.0`, 27015));";
    }
}
