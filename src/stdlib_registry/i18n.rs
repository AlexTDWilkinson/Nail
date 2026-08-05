//! I18n module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, I18n:
        "i18n_load" [SerdeJson] => "std_lib::i18n::load", (directory: s) -> (i!e),
            "Loads every .json message catalog in a directory, once at startup. The file stem is the locale - en.json holds `en`, pt-BR.json holds `pt-BR` - and each file is one flat object of key to text. Returns how many messages were loaded.",
            "loaded:i = danger(i18n_load(`locales`));";
        "i18n_translate" => "std_lib::i18n::translate", (locale: s, key: s) -> s,
            "The message for a key in a locale. `pt-BR` falls back to `pt`, then to `en`, and a key nobody defines comes back as itself - visible in the page and greppable, never a crash.",
            "title:s = i18n_translate(user_locale, `menu.title`);";
        "i18n_translate_count" => "std_lib::i18n::translate_count", (locale: s, key: s, count: i) -> s,
            "The message for a count: `<key>.one` when the count is 1, `<key>.other` otherwise, with {count} in the text replaced by the number.",
            "summary:s = i18n_translate_count(user_locale, `items`, cart_size);";
        "i18n_locales" => "std_lib::i18n::locales", () -> [s],
            "Every locale with a loaded catalog, for language pickers.",
            "choices:a:s = i18n_locales();";
    }
}
