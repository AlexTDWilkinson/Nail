//! Diff module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Diff:
        "diff_lines" [Diffy] => "std_lib::diff::lines", (old: s, new: s) -> s,
            "The unified diff between two texts - the format git and code review read. Two equal texts diff to an empty patch body.",
            "saved_config:s = `port = 8080\\ndebug = false`;\nedited_config:s = `port = 9090\\ndebug = false`;\npatch:s = diff_lines(saved_config, edited_config);";
        "diff_apply" [Diffy] => "std_lib::diff::apply", (text: s, patch: s) -> (s!e),
            "Applies a patch diff_lines made (or git did) to a text, giving the new text. A patch that does not fit says where it failed instead of guessing.",
            "saved_config:s = `port = 8080\\ndebug = false`;\nedited_config:s = `port = 9090\\ndebug = false`;\npatch:s = diff_lines(saved_config, edited_config);\nupdated:s = danger(diff_apply(saved_config, patch));";
        "diff_changed" => "std_lib::diff::changed", (old: s, new: s) -> b,
            "Whether two texts differ at all - cheaper to ask than to render the diff.",
            "saved_config:s = `port = 8080\\ndebug = false`;\nedited_config:s = `port = 9090\\ndebug = false`;\ndirty:b = diff_changed(saved_config, edited_config);";
    }
}
