//! Template module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Template:
        "template_render" => "std_lib::template::render", (template: s, values: (h s s)) -> (s!e),
            "Fills the values into the template, escaping each one for HTML. {{name}} is escaped, {{{name}}} is raw, {{#if name}}...{{else}}...{{/if}} and {{#unless name}}...{{/unless}} choose a part, and {{! }} is a comment. A name the values do not have is an error.",
            "layout:s = `Hello {{user_name}}, you have {{count}} messages.`;\nvalues:h<s,s> = hashmap_new();\nhashmap_set(values, `user_name`, `Ada`);\nhashmap_set(values, `count`, `3`);\npage:s = danger(template_render(layout, values));";
        "template_render_rows" => "std_lib::template::render_rows", (template: s, rows: [(h s s)]) -> (s!e),
            "Renders the same template once for each set of values and joins the results, which is how a table body or a list of cards is built.",
            "row_template:s = `<li>{{name}}</li>`;\nfirst:h<s,s> = hashmap_new();\nhashmap_set(first, `name`, `Ada`);\nrows:a:h<s,s> = [first];\nbody:s = danger(template_render_rows(row_template, rows));";
        "template_names_used" => "std_lib::template::names_used", (template: s) -> ([s]!e),
            "Returns the names a template asks for, so a program can check it holds them before rendering.",
            "layout:s = `Hello {{user_name}}, you have {{count}} messages.`;\nneeded:a:s = danger(template_names_used(layout));";
        "template_has" => "std_lib::template::has", (template: s, name: s) -> (b!e),
            "Returns whether the template mentions the named placeholder, in a value tag or as the name a conditional asks about. A template that does not mention it is false, and a template that cannot be read is an error.",
            "layout:s = `Hello {{user_name}}, you have {{count}} messages.`;\npersonalised:b = danger(template_has(layout, `user_name`));";
        "template_render_or" => "std_lib::template::render_or", (template: s, values: (h s s), fallback: s) -> (s!e),
            "Fills the values into the template like template_render, except that a name the values do not have becomes the fallback text instead of an error. A template that cannot be read is still an error, because no fallback repairs a tag that is never closed.",
            "layout:s = `Hello {{user_name}}, you have {{count}} messages.`;\nvalues:h<s,s> = hashmap_new();\nhashmap_set(values, `user_name`, `Ada`);\nhashmap_set(values, `count`, `3`);\npage:s = danger(template_render_or(layout, values, `unknown`));";
    }
}
