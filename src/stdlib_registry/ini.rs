//! INI module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Ini:
        "ini_get" => "std_lib::ini::get", (text: s, section: s, key: s) -> (s!e),
            "The value under a section, with an empty section name meaning the top of the file. Inline comments are stripped, values trimmed, quoted values unquoted, and both `=` and `:` separate. A missing section or key is named in the error.",
            "config:s = `[server]\\nport = 8080\\ndebug = true`;\nport:s = danger(ini_get(config, `server`, `port`));";
        "ini_sections" => "std_lib::ini::sections", (text: s) -> [s],
            "Section header names in order of first appearance, without duplicates.",
            "config:s = `[server]\\nport = 8080\\ndebug = true`;\nnames:a:s = ini_sections(config);";
        "ini_keys" => "std_lib::ini::keys", (text: s, section: s) -> ([s]!e),
            "The keys of one section in order, with an empty section name meaning the top of the file. A missing section is named in the error.",
            "config:s = `[server]\\nport = 8080\\ndebug = true`;\nsettings:a:s = danger(ini_keys(config, `server`));";
        "ini_has" => "std_lib::ini::has", (text: s, section: s, key: s) -> b,
            "Whether the section holds the key.",
            "config:s = `[server]\\nport = 8080\\ndebug = true`;\nknown:b = ini_has(config, `server`, `port`);";
        "ini_set" => "std_lib::ini::set", (text: s, section: s, key: s, value: s) -> s,
            "The text with the key set, replaced in place so order and comments survive. An absent key is appended to its section and an absent section is created at the end.",
            "config:s = `[server]\\nport = 8080\\ndebug = true`;\nupdated:s = ini_set(config, `server`, `port`, `9090`);";
        "ini_remove" => "std_lib::ini::remove", (text: s, section: s, key: s) -> s,
            "The text with the key removed. Removing what is absent returns the text unchanged.",
            "config:s = `[server]\\nport = 8080\\ndebug = true`;\nsmaller:s = ini_remove(config, `server`, `debug`);";
    }
}
