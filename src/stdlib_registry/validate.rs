//! Validate module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Validate:
        "validate_email" => "std_lib::validate::email", (text: (&s)) -> b,
            "Returns true if the text is an email address that could be delivered to.",
            "submitted_address:s = `ada@example.com`;\nusable:b = validate_email(submitted_address);";
        "validate_url" => "std_lib::validate::url", (text: (&s)) -> b,
            "Returns true if the text is a URL with a scheme and a host.",
            "submitted_link:s = `https://nail-lang.org`;\nlinkable:b = validate_url(submitted_link);";
        "validate_hostname" => "std_lib::validate::hostname", (text: (&s)) -> b,
            "Returns true if the text is a hostname: dot-separated labels of letters, digits and hyphens.",
            "config_host:s = `db.internal`;\nresolvable:b = validate_hostname(config_host);";
        "validate_uuid" => "std_lib::validate::uuid", (text: (&s)) -> b,
            "Returns true if the text is a UUID in the usual 8-4-4-4-12 spelling.",
            "request_id:s = `6ba7b810-9dad-11d1-80b4-00c04fd430c8`;\nidentifier:b = validate_uuid(request_id);";
        "validate_ipv4" => "std_lib::validate::ipv4", (text: (&s)) -> b,
            "Returns true if the text is an IPv4 address.",
            "bind_address:s = `127.0.0.1`;\nnumeric_host:b = validate_ipv4(bind_address);";
        "validate_ipv6" => "std_lib::validate::ipv6", (text: (&s)) -> b,
            "Returns true if the text is an IPv6 address.",
            "bind_address:s = `::1`;\nnumeric_host:b = validate_ipv6(bind_address);";
        "validate_port" => "std_lib::validate::port", (number: i) -> b,
            "Returns true if the number is a port a program could bind or connect to, so 1 through 65535.",
            "configured_port:i = 8080;\nbindable:b = validate_port(configured_port);";
        "validate_credit_card" => "std_lib::validate::credit_card", (text: (&s)) -> b,
            "Returns true if the digits pass the Luhn check, catching a mistyped card number before a payment is attempted.",
            "card_number:s = `4111 1111 1111 1111`;\ntyped_correctly:b = validate_credit_card(card_number);";
        "validate_hex_color" => "std_lib::validate::hex_color", (text: (&s)) -> b,
            "Returns true if the text is a hash followed by three, four, six or eight hex digits.",
            "theme_colour:s = `#1b2a4a`;\npaintable:b = validate_hex_color(theme_colour);";
        "validate_slug" => "std_lib::validate::slug", (text: (&s)) -> b,
            "Returns true if the text is a slug: lowercase letters, digits and single hyphens, with no hyphen at either end.",
            "url_part:s = `getting-started`;\nlinkable:b = validate_slug(url_part);";
        "validate_length_between" => "std_lib::validate::length_between", (text: (&s), minimum: i, maximum: i) -> b,
            "Returns true if the text has between the given numbers of characters.",
            "display_name:s = `ada`;\nacceptable:b = validate_length_between(display_name, 2, 32);";
        "validate_password_strength" => "std_lib::validate::password_strength", (text: (&s)) -> i,
            "Returns how strong a password is from 0 to 4, scoring length and variety and giving nothing to the passwords everybody tries first.",
            "chosen_password:s = `correct horse battery staple`;\nstrength:i = validate_password_strength(chosen_password);";
        "validate_json" [SerdeJson] => "std_lib::validate::json", (text: (&s)) -> b,
            "Returns true if the text is a JSON document, answered by parsing it.",
            "body:s = `{\"name\":\"Ada\"}`;\nreadable:b = validate_json(body);";
        "validate_luhn" => "std_lib::validate::luhn", (digits: (&s)) -> b,
            "Returns true if the digits pass the bare Luhn checksum, whatever their length - IMEIs and other identifiers as well as card numbers. Spaces and hyphens are ignored.",
            "device_imei:s = `490154203237518`;\ntyped_correctly:b = validate_luhn(device_imei);";
        "validate_iban" => "std_lib::validate::iban", (text: (&s)) -> b,
            "Returns true if the text is an IBAN: the right length for its country and passing the mod-97 check. Spaces are ignored. Knows the common European countries. Anywhere else is false.",
            "account_number:s = `GB82 WEST 1234 5698 7654 32`;\npayable:b = validate_iban(account_number);";
        "validate_mac_address" => "std_lib::validate::mac_address", (text: (&s)) -> b,
            "Returns true if the text is a MAC address: six pairs of hex digits separated by colons or dashes.",
            "interface_id:s = `3c:22:fb:aa:11:02`;\nhardware:b = validate_mac_address(interface_id);";
        "validate_phone_loose" => "std_lib::validate::phone_loose", (text: (&s)) -> b,
            "Returns true if the text is 7 to 15 digits once the +, spaces, dashes, parentheses and dots people format numbers with are stripped - the sanity check a signup form wants.",
            "submitted_number:s = `+1 604 555 0134`;\ncallable:b = validate_phone_loose(submitted_number);";
        "validate_isbn" => "std_lib::validate::isbn", (text: (&s)) -> b,
            "Returns true if the text is an ISBN-10 or ISBN-13 with a correct checksum. Hyphens and spaces are ignored.",
            "book_number:s = `978-0-13-235088-4`;\norderable:b = validate_isbn(book_number);";
        "validate_schema" [JsonSchema, SerdeJson] => "std_lib::validate::schema", (json: s, schema: s) -> ([s]!e),
            "Checks a JSON document against a JSON Schema - types, ranges, required fields, formats. The answer is the list of problems with the path where each sits. An empty list means the document passes. The error case is a schema or document that does not even parse.",
            "body:s = `{\"total\":42}`;\norder_schema:s = `{\"type\":\"object\",\"required\":[\"total\"]}`;\nproblems:a:s = danger(validate_schema(body, order_schema));";
    }

    // validate_postal_code takes the VALIDATE_Country enum, which needs a
    // custom type import, so it uses the full struct form.
    m.insert("validate_postal_code", StdlibFunction {
        rust_path: "std_lib::validate::postal_code".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("VALIDATE_Country", "nail::std_lib::validate")],
        module: StdlibModule::Validate,
        parameters: vec![
            nail_param!(text: (&s)),
            StdlibParameter { name: "country".to_string(), param_type: NailDataTypeDescriptor::Enum("VALIDATE_Country".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!(b),
        diverging: false,
        description: "Returns whether the text is a postal code shaped the way the given country shapes them. The country is a VALIDATE_Country variant, so there is no unknown country to be wrong about and the answer is a plain boolean.",
        example: "form_code:s = `V6B 1A1`;\ndeliverable:b = validate_postal_code(form_code, VALIDATE_Country::Canada);",
    });
}
