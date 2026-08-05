//! Validate module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Validate:
        "validate_email" => "std_lib::validate::email", (text: (&s)) -> b,
            "Returns true if the text is an email address that could be delivered to.",
            "usable:b = validate_email(submitted_address);";
        "validate_url" => "std_lib::validate::url", (text: (&s)) -> b,
            "Returns true if the text is a URL with a scheme and a host.",
            "linkable:b = validate_url(submitted_link);";
        "validate_hostname" => "std_lib::validate::hostname", (text: (&s)) -> b,
            "Returns true if the text is a hostname: dot-separated labels of letters, digits and hyphens.",
            "resolvable:b = validate_hostname(config_host);";
        "validate_uuid" => "std_lib::validate::uuid", (text: (&s)) -> b,
            "Returns true if the text is a UUID in the usual 8-4-4-4-12 spelling.",
            "identifier:b = validate_uuid(request_id);";
        "validate_ipv4" => "std_lib::validate::ipv4", (text: (&s)) -> b,
            "Returns true if the text is an IPv4 address.",
            "numeric_host:b = validate_ipv4(bind_address);";
        "validate_ipv6" => "std_lib::validate::ipv6", (text: (&s)) -> b,
            "Returns true if the text is an IPv6 address.",
            "numeric_host:b = validate_ipv6(bind_address);";
        "validate_port" => "std_lib::validate::port", (number: i) -> b,
            "Returns true if the number is a port a program could bind or connect to, so 1 through 65535.",
            "bindable:b = validate_port(configured_port);";
        "validate_credit_card" => "std_lib::validate::credit_card", (text: (&s)) -> b,
            "Returns true if the digits pass the Luhn check, catching a mistyped card number before a payment is attempted.",
            "typed_correctly:b = validate_credit_card(card_number);";
        "validate_hex_color" => "std_lib::validate::hex_color", (text: (&s)) -> b,
            "Returns true if the text is a hash followed by three, four, six or eight hex digits.",
            "paintable:b = validate_hex_color(theme_colour);";
        "validate_slug" => "std_lib::validate::slug", (text: (&s)) -> b,
            "Returns true if the text is a slug: lowercase letters, digits and single hyphens, with no hyphen at either end.",
            "linkable:b = validate_slug(url_part);";
        "validate_length_between" => "std_lib::validate::length_between", (text: (&s), minimum: i, maximum: i) -> b,
            "Returns true if the text has between the given numbers of characters.",
            "acceptable:b = validate_length_between(display_name, 2, 32);";
        "validate_password_strength" => "std_lib::validate::password_strength", (text: (&s)) -> i,
            "Returns how strong a password is from 0 to 4, scoring length and variety and giving nothing to the passwords everybody tries first.",
            "strength:i = validate_password_strength(chosen_password);";
        "validate_json" [SerdeJson] => "std_lib::validate::json", (text: (&s)) -> b,
            "Returns true if the text is a JSON document, answered by parsing it.",
            "readable:b = validate_json(request.body);";
    }
}
