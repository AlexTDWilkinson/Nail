//! Sending mail.
//!
//! A program that has accounts in it has to send mail - a confirmation, a
//! password reset, an alert when something breaks - and there is no way to do
//! that without an SMTP server. So this takes the details of one, sends a
//! message through it, and reports what the server said if it refused.
//!
//! Only sending is here. Reading mail means IMAP, a mailbox full of MIME parts,
//! and attachments that are not text, none of which a Nail program has any
//! business doing.
//!
//! The server details go in a struct rather than eight arguments, because they
//! come from configuration as a unit and are passed around as one - and because
//! a password is easier to keep out of a log when it is a field on a value
//! rather than the third string in a call.

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};

/// How to reach a mail server, and who the mail is from.
///
/// `port` 587 with `use_tls` true is what almost every provider wants:
/// STARTTLS on the submission port. Port 465 is TLS from the first byte, which
/// `use_tls` also covers. Port 25 with `use_tls` false reaches a server on the
/// same machine, and nothing else - no provider on the internet accepts it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EMAIL_Server {
    pub host: String,
    pub port: i64,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: String,
    pub use_tls: bool,
}

/// The details of a mail server, filled in with what almost every provider
/// wants, so a program only overrides what is actually different.
pub fn default_server() -> EMAIL_Server {
    return EMAIL_Server { host: String::new(), port: 587, username: String::new(), password: String::new(), from_address: String::new(), from_name: String::new(), use_tls: true };
}

/// Builds the sender, checking the details before any connection is attempted so
/// a missing host is a clear error rather than a name resolution failure.
fn transport(server: &EMAIL_Server, function_name: &str) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
    if server.host.is_empty() {
        return Err(format!("{}: the server's host is empty, so there is nowhere to send the mail", function_name));
    }
    if !(1..=65535).contains(&server.port) {
        return Err(format!("{}: {} is not a port", function_name, server.port));
    }
    if server.from_address.is_empty() {
        return Err(format!("{}: the server's from_address is empty, and mail cannot be sent without a sender", function_name));
    }

    let mut builder = if server.use_tls {
        // STARTTLS on 587 and implicit TLS on 465 are different handshakes, and
        // which one a server wants is decided by the port it listens on.
        if server.port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&server.host).map_err(|failure| format!("{}: could not set up a connection to {}: {}", function_name, server.host, failure))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&server.host).map_err(|failure| format!("{}: could not set up a connection to {}: {}", function_name, server.host, failure))?
        }
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&server.host)
    };
    builder = builder.port(server.port as u16);

    // An empty username means an unauthenticated server, which is only ever the
    // one running on the same machine.
    if !server.username.is_empty() {
        builder = builder.credentials(Credentials::new(server.username.clone(), server.password.clone()));
    }
    return Ok(builder.build());
}

fn build_message(server: &EMAIL_Server, to: &str, subject: &str, body: &str, content_type: ContentType, function_name: &str) -> Result<Message, String> {
    let from = if server.from_name.is_empty() { server.from_address.clone() } else { format!("{} <{}>", server.from_name, server.from_address) };
    return Message::builder()
        .from(from.parse().map_err(|failure| format!("{}: `{}` is not an address mail can be sent from: {}", function_name, from, failure))?)
        .to(to.parse().map_err(|failure| format!("{}: `{}` is not an address mail can be sent to: {}", function_name, to, failure))?)
        .subject(subject)
        .header(content_type)
        .body(body.to_string())
        .map_err(|failure| format!("{}: the message could not be assembled: {}", function_name, failure));
}

/// Sends a plain text message and waits for the server to accept it. Success
/// means the server took the message, not that it was delivered - what happens
/// after that is between mail servers.
pub async fn send(server: EMAIL_Server, to: String, subject: String, body: String) -> Result<(), String> {
    let message = build_message(&server, &to, &subject, &body, ContentType::TEXT_PLAIN, "email_send")?;
    let sender = transport(&server, "email_send")?;
    sender.send(message).await.map_err(|failure| format!("email_send: {} refused the message: {}", server.host, failure))?;
    return Ok(());
}

/// Sends an HTML message. Mail readers are far stricter than browsers - many
/// ignore stylesheets entirely - so the HTML here should be plain markup with
/// inline styles, not a page.
pub async fn send_html(server: EMAIL_Server, to: String, subject: String, html: String) -> Result<(), String> {
    let message = build_message(&server, &to, &subject, &html, ContentType::TEXT_HTML, "email_send_html")?;
    let sender = transport(&server, "email_send_html")?;
    sender.send(message).await.map_err(|failure| format!("email_send_html: {} refused the message: {}", server.host, failure))?;
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_server() -> EMAIL_Server {
        return EMAIL_Server {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "postmaster@example.com".to_string(),
            password: "a-password".to_string(),
            from_address: "hello@example.com".to_string(),
            from_name: "Example".to_string(),
            use_tls: true,
        };
    }

    #[test]
    fn the_defaults_are_what_a_provider_wants() {
        let defaults = default_server();
        assert_eq!(defaults.port, 587);
        assert!(defaults.use_tls);
    }

    #[test]
    fn a_message_carries_the_sender_the_server_was_given() {
        let message = build_message(&a_server(), "someone@example.com", "Hello", "text", ContentType::TEXT_PLAIN, "email_send").expect("a sendable message");
        let written = String::from_utf8(message.formatted()).expect("text");
        assert!(written.contains("From: Example <hello@example.com>"), "got: {}", written);
        assert!(written.contains("To: someone@example.com"), "got: {}", written);
        assert!(written.contains("Subject: Hello"), "got: {}", written);
    }

    #[test]
    fn a_server_with_no_from_name_sends_from_the_address_alone() {
        let mut server = a_server();
        server.from_name = String::new();
        let message = build_message(&server, "someone@example.com", "Hello", "text", ContentType::TEXT_PLAIN, "email_send").expect("a sendable message");
        let written = String::from_utf8(message.formatted()).expect("text");
        assert!(written.contains("From: hello@example.com"), "got: {}", written);
    }

    #[test]
    fn html_and_text_messages_are_marked_as_what_they_are() {
        let server = a_server();
        let text = String::from_utf8(build_message(&server, "someone@example.com", "s", "body", ContentType::TEXT_PLAIN, "email_send").expect("a message").formatted()).expect("text");
        assert!(text.contains("text/plain"), "got: {}", text);
        let html = String::from_utf8(build_message(&server, "someone@example.com", "s", "<p>body</p>", ContentType::TEXT_HTML, "email_send_html").expect("a message").formatted()).expect("text");
        assert!(html.contains("text/html"), "got: {}", html);
    }

    #[test]
    fn an_address_that_is_not_an_address_is_refused_before_connecting() {
        let failure = build_message(&a_server(), "not an address", "s", "body", ContentType::TEXT_PLAIN, "email_send").unwrap_err();
        assert!(failure.contains("is not an address mail can be sent to"), "got: {}", failure);
    }

    /// Details that cannot work are refused without a connection attempt, so the
    /// error names what is wrong rather than reporting a failed lookup.
    #[test]
    fn missing_server_details_are_refused_before_connecting() {
        let mut without_host = a_server();
        without_host.host = String::new();
        assert!(transport(&without_host, "email_send").unwrap_err().contains("nowhere to send"));

        let mut without_sender = a_server();
        without_sender.from_address = String::new();
        assert!(transport(&without_sender, "email_send").unwrap_err().contains("cannot be sent without a sender"));

        let mut bad_port = a_server();
        bad_port.port = 0;
        assert!(transport(&bad_port, "email_send").unwrap_err().contains("is not a port"));
    }

    #[test]
    fn a_usable_server_builds_a_sender() {
        assert!(transport(&a_server(), "email_send").is_ok());
        let mut implicit_tls = a_server();
        implicit_tls.port = 465;
        assert!(transport(&implicit_tls, "email_send").is_ok());
        let mut local = a_server();
        local.port = 25;
        local.use_tls = false;
        local.username = String::new();
        assert!(transport(&local, "email_send").is_ok());
    }
}
