//! Email module stdlib registry entries.
//!
//! `email_send` and `email_send_html` take the server as a struct, which
//! `simple_fns!` cannot express, so both are written out in full.

use super::*;

/// The two send functions differ only in how the body is marked, so they are
/// built from one description rather than written twice.
fn send_fn(rust_path: &str, body_name: &str, description: &'static str, example: &'static str) -> StdlibFunction {
    return StdlibFunction {
        rust_path: rust_path.to_string(),
        crate_deps: vec![CrateDependency::Lettre, CrateDependency::Tokio, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("EMAIL_Server", "nail::std_lib::email")],
        module: StdlibModule::Email,
        parameters: vec![
            StdlibParameter { name: "server".to_string(), param_type: NailDataTypeDescriptor::Struct("EMAIL_Server".to_string()), pass_by_reference: false },
            nail_param!(to: s),
            nail_param!(subject: s),
            StdlibParameter { name: body_name.to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
        ],
        return_type: nail_type!((v!e)),
        diverging: false,
        description,
        example,
    };
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("email_send_with_attachments", StdlibFunction {
        rust_path: "std_lib::email::send_with_attachments".to_string(),
        crate_deps: vec![CrateDependency::Lettre, CrateDependency::Tokio, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("EMAIL_Server", "nail::std_lib::email"), ("EMAIL_Attachment", "nail::std_lib::email")],
        module: StdlibModule::Email,
        parameters: vec![
            StdlibParameter { name: "server".to_string(), param_type: NailDataTypeDescriptor::Struct("EMAIL_Server".to_string()), pass_by_reference: false },
            nail_param!(to: s),
            nail_param!(subject: s),
            nail_param!(body: s),
            StdlibParameter { name: "attachments".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("EMAIL_Attachment".to_string()))), pass_by_reference: false },
        ],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Sends a plain text message with files attached - the invoice, the export, the report just made. An attachment's empty file_name shows the reader the file's own name, and an empty mime_type is guessed from the extension.",
        example: "danger(email_send_with_attachments(server, to, `Your invoice`, body, [EMAIL_Attachment { path = invoice_path, file_name = ``, mime_type = `` }]));",
    });

    m.insert("email_default_server", StdlibFunction {
        rust_path: "std_lib::email::default_server".to_string(),
        crate_deps: vec![CrateDependency::Lettre, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("EMAIL_Server", "nail::std_lib::email")],
        module: StdlibModule::Email,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("EMAIL_Server".to_string()),
        diverging: false,
        description: "The details of a mail server filled in with what almost every provider wants - port 587 with TLS - so a program only sets what is different.",
        example: "server:EMAIL_Server = email_default_server();",
    });

    m.insert(
        "email_send",
        send_fn(
            "std_lib::email::send",
            "body",
            "Sends a plain text message through an SMTP server and waits for it to be accepted. Success means the server took the message, not that it was delivered.",
            "danger(email_send(server, `someone@example.com`, `Your receipt`, body));",
        ),
    );

    m.insert(
        "email_send_html",
        send_fn(
            "std_lib::email::send_html",
            "html",
            "Sends an HTML message. Mail readers are far stricter than browsers, so this wants plain markup with inline styles rather than a page.",
            "danger(email_send_html(server, `someone@example.com`, `Your receipt`, markup));",
        ),
    );
}
