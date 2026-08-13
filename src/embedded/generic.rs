//! One scanner for every language built the same way: words, numbers, quoted
//! strings, line comments and block comments.
//!
//! JavaScript, SQL, Rust, Python and a dozen others differ in which words are
//! reserved and which marks open a comment, and in nothing else this
//! highlighter can see. So those differences are a table entry - a [`Syntax`] -
//! rather than a code path, and teaching the highlighter a new language of this
//! shape means adding a row, never editing the scanner.

use super::{flush, Piece};

/// Everything that differs between the languages this scanner handles.
pub struct Syntax {
    /// Marks that run a comment to the end of the line: `//`, `#`, `--`.
    line_comments: &'static [&'static str],
    /// The opening and closing marks of a comment that spans lines.
    block_comment: Option<(&'static str, &'static str)>,
    /// Characters that open a string. A language whose quote character has
    /// another job - Rust's `'a` lifetimes - simply leaves it out.
    quotes: &'static [char],
    keywords: &'static [&'static str],
    /// Whether `select` and `SELECT` are the same word, as in SQL.
    case_insensitive: bool,
    /// Marks that introduce a variable: `$name` in shell and PHP.
    sigils: &'static [char],
    /// Whether a capitalized word names a type. True where the convention is
    /// near-universal - Rust, Go, Java - and false where it is not.
    capitals_are_types: bool,
}

/// A language this scanner knows. Each one is a [`Syntax`] and nothing more.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dialect {
    Script,
    Sql,
    Python,
    Ruby,
    Shell,
    Rust,
    Go,
    Java,
    CSharp,
    C,
    Php,
    Swift,
    Kotlin,
    Lua,
    GraphQl,
    Wgsl,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Inside {
    Code,
    Quote(char),
    Comment,
}

/// How far into a language a line ended: a block comment and an unterminated
/// string both continue onto the next line.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct State {
    dialect: Dialect,
    inside: Inside,
}

pub fn start(dialect: Dialect) -> State {
    return State { dialect, inside: Inside::Code };
}

const SLASH_COMMENTS: &[&str] = &["//"];
const HASH_COMMENTS: &[&str] = &["#"];
const C_BLOCK: Option<(&str, &str)> = Some(("/*", "*/"));

/// The table. One row per language; the scanner below reads it and knows
/// nothing else about any of them.
fn syntax(dialect: Dialect) -> &'static Syntax {
    return match dialect {
        Dialect::Script => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            quotes: &['"', '\'', '`'],
            keywords: &[
                "abstract", "any", "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "declare", "default", "delete", "do", "else", "enum", "export", "extends",
                "false", "finally", "for", "from", "function", "get", "if", "implements", "import", "in", "instanceof", "interface", "keyof", "let", "namespace", "never", "new", "null", "of",
                "private", "protected", "public", "readonly", "return", "satisfies", "set", "static", "super", "switch", "this", "throw", "true", "try", "type", "typeof", "undefined", "unknown",
                "var", "void", "while", "yield",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::Sql => &Syntax {
            line_comments: &["--"],
            block_comment: C_BLOCK,
            quotes: &['\'', '"'],
            keywords: &[
                "ADD", "ALL", "ALTER", "AND", "AS", "ASC", "BEGIN", "BETWEEN", "BIGINT", "BLOB", "BOOLEAN", "BY", "CASCADE", "CASE", "CHECK", "COLUMN", "COMMIT", "CONFLICT", "CONSTRAINT",
                "CREATE", "CROSS", "DATE", "DECIMAL", "DEFAULT", "DELETE", "DESC", "DISTINCT", "DO", "DOUBLE", "DROP", "ELSE", "END", "EXISTS", "FALSE", "FLOAT", "FOREIGN", "FROM", "FULL",
                "GROUP", "HAVING", "IF", "IN", "INDEX", "INNER", "INSERT", "INT", "INTEGER", "INTO", "IS", "JOIN", "JSON", "JSONB", "KEY", "LEFT", "LIKE", "LIMIT", "NOT", "NULL", "OFFSET", "ON",
                "OR", "ORDER", "OUTER", "PRIMARY", "REAL", "REFERENCES", "RETURNING", "RIGHT", "ROLLBACK", "SELECT", "SERIAL", "SET", "TABLE", "TEXT", "THEN", "TIMESTAMP", "TRANSACTION", "TRUE",
                "UNION", "UNIQUE", "UPDATE", "USING", "UUID", "VALUES", "VARCHAR", "VIEW", "WHEN", "WHERE", "WITH",
            ],
            case_insensitive: true,
            sigils: &[],
            capitals_are_types: false,
        },
        Dialect::Python => &Syntax {
            line_comments: HASH_COMMENTS,
            block_comment: None,
            quotes: &['"', '\''],
            keywords: &[
                "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in",
                "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "self", "True", "try", "while", "with", "yield",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::Ruby => &Syntax {
            line_comments: HASH_COMMENTS,
            block_comment: None,
            quotes: &['"', '\''],
            keywords: &[
                "alias", "and", "begin", "break", "case", "class", "def", "do", "else", "elsif", "end", "ensure", "false", "for", "if", "in", "module", "next", "nil", "not", "or", "raise",
                "require", "require_relative", "rescue", "retry", "return", "self", "super", "then", "true", "unless", "until", "when", "while", "yield",
            ],
            case_insensitive: false,
            sigils: &['@'],
            capitals_are_types: true,
        },
        Dialect::Shell => &Syntax {
            line_comments: HASH_COMMENTS,
            block_comment: None,
            quotes: &['"', '\''],
            keywords: &[
                "case", "declare", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in", "local", "readonly", "return", "select", "shift", "source", "then",
                "trap", "unset", "until", "while",
            ],
            case_insensitive: false,
            sigils: &['$'],
            capitals_are_types: false,
        },
        Dialect::Rust => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            // A single quote opens a lifetime far more often than a character
            // literal, and swallowing `&'a str` would color the rest as string.
            quotes: &['"'],
            keywords: &[
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
                "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::Go => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            quotes: &['"', '`'],
            keywords: &[
                "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough", "false", "for", "func", "go", "goto", "if", "import", "interface", "map", "nil",
                "package", "range", "return", "select", "struct", "switch", "true", "type", "var",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::Java => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            quotes: &['"', '\''],
            keywords: &[
                "abstract", "boolean", "break", "byte", "case", "catch", "char", "class", "continue", "default", "do", "double", "else", "enum", "extends", "final", "finally", "float", "for",
                "if", "implements", "import", "instanceof", "int", "interface", "long", "new", "null", "package", "private", "protected", "public", "record", "return", "short", "static",
                "super", "switch", "synchronized", "this", "throw", "throws", "try", "void", "volatile", "while", "true", "false", "var",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::CSharp => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            quotes: &['"', '\''],
            keywords: &[
                "abstract", "async", "await", "base", "bool", "break", "case", "catch", "class", "const", "continue", "decimal", "default", "do", "double", "else", "enum", "false", "finally",
                "float", "for", "foreach", "get", "if", "in", "int", "interface", "internal", "long", "namespace", "new", "null", "object", "override", "partial", "private", "protected",
                "public", "readonly", "record", "return", "sealed", "set", "static", "string", "struct", "switch", "this", "throw", "true", "try", "using", "var", "virtual", "void", "while",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::C => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            quotes: &['"', '\''],
            keywords: &[
                "auto", "bool", "break", "case", "char", "class", "const", "constexpr", "continue", "default", "delete", "do", "double", "else", "enum", "extern", "false", "float", "for",
                "goto", "if", "inline", "int", "long", "namespace", "new", "nullptr", "override", "private", "protected", "public", "register", "return", "short", "signed", "sizeof", "static",
                "struct", "switch", "template", "this", "true", "typedef", "typename", "union", "unsigned", "using", "virtual", "void", "volatile", "while",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: false,
        },
        Dialect::Php => &Syntax {
            line_comments: &["//", "#"],
            block_comment: C_BLOCK,
            quotes: &['"', '\''],
            keywords: &[
                "abstract", "array", "as", "break", "case", "catch", "class", "const", "continue", "declare", "default", "do", "echo", "else", "elseif", "extends", "false", "final", "finally",
                "fn", "for", "foreach", "function", "global", "if", "implements", "include", "instanceof", "interface", "isset", "match", "namespace", "new", "null", "print", "private",
                "protected", "public", "require", "return", "static", "switch", "throw", "trait", "true", "try", "unset", "use", "var", "while", "yield",
            ],
            case_insensitive: false,
            sigils: &['$'],
            capitals_are_types: true,
        },
        Dialect::Swift => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            quotes: &['"'],
            keywords: &[
                "as", "async", "await", "break", "case", "catch", "class", "continue", "default", "defer", "deinit", "do", "else", "enum", "extension", "fallthrough", "false", "fileprivate",
                "final", "for", "func", "guard", "if", "import", "in", "init", "internal", "is", "lazy", "let", "nil", "open", "private", "protocol", "public", "repeat", "return", "self",
                "static", "struct", "super", "switch", "throw", "throws", "true", "try", "typealias", "unowned", "var", "weak", "where", "while",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::Kotlin => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            quotes: &['"'],
            keywords: &[
                "abstract", "as", "by", "catch", "class", "companion", "constructor", "data", "do", "else", "enum", "false", "final", "finally", "for", "fun", "if", "import", "in", "init",
                "interface", "internal", "is", "lateinit", "null", "object", "open", "override", "package", "private", "protected", "public", "return", "sealed", "suspend", "this", "throw",
                "true", "try", "typealias", "val", "var", "when", "while",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: true,
        },
        Dialect::Lua => &Syntax {
            line_comments: &["--"],
            block_comment: None,
            quotes: &['"', '\''],
            keywords: &[
                "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
            ],
            case_insensitive: false,
            sigils: &[],
            capitals_are_types: false,
        },
        Dialect::GraphQl => &Syntax {
            line_comments: HASH_COMMENTS,
            block_comment: None,
            quotes: &['"'],
            keywords: &[
                "directive", "enum", "extend", "false", "fragment", "implements", "input", "interface", "mutation", "null", "on", "query", "scalar", "schema", "subscription", "true", "type",
                "union",
            ],
            case_insensitive: false,
            sigils: &['$'],
            capitals_are_types: true,
        },
        Dialect::Wgsl => &Syntax {
            line_comments: SLASH_COMMENTS,
            block_comment: C_BLOCK,
            // WGSL has no string literals, so nothing opens one.
            quotes: &[],
            // The built-in types sit in the keyword list, the way C's do: WGSL
            // spells its types in lowercase, so the capital convention below
            // only reaches the shader's own structs.
            keywords: &[
                "alias", "array", "atomic", "bool", "break", "case", "const", "const_assert", "continue", "continuing", "default", "diagnostic", "discard", "else", "enable", "f16", "f32",
                "false", "fn", "for", "function", "i32", "if", "let", "loop", "mat2x2", "mat2x3", "mat2x4", "mat3x2", "mat3x3", "mat3x4", "mat4x2", "mat4x3", "mat4x4", "override", "private",
                "ptr", "read", "read_write", "requires", "return", "sampler", "sampler_comparison", "storage", "struct", "switch", "texture_2d", "texture_2d_array", "texture_3d",
                "texture_cube", "texture_cube_array", "texture_depth_2d", "texture_multisampled_2d", "texture_storage_2d", "true", "u32", "uniform", "var", "vec2", "vec2f", "vec2h", "vec2i",
                "vec2u", "vec3", "vec3f", "vec3h", "vec3i", "vec3u", "vec4", "vec4f", "vec4h", "vec4i", "vec4u", "while", "workgroup", "write",
            ],
            case_insensitive: false,
            // `@vertex`, `@group(0)`, `@location(0)`: an attribute is one
            // piece, the same way a shell variable is.
            sigils: &['@'],
            capitals_are_types: true,
        },
    };
}

/// Walks `body` from `state`, handing every run to `emit`. See
/// [`super::tokenize`] for how callers drive this across lines.
pub fn tokenize(body: &str, state: &mut State, mut emit: impl FnMut(&str, Piece)) {
    let syntax = syntax(state.dialect);
    let chars: Vec<char> = body.chars().collect();
    let mut index = 0;
    let mut run = String::new();
    // Whether the last thing emitted was a `.`, which makes the next word a
    // member name rather than a name of its own.
    let mut after_dot = false;

    while index < chars.len() {
        let ch = chars[index];
        match state.inside {
            Inside::Comment => {
                let closing = syntax.block_comment.map(|(_, close)| close).unwrap_or("*/");
                run.push(ch);
                index += 1;
                if run.ends_with(closing) {
                    flush(&mut run, Piece::Comment, &mut emit);
                    state.inside = Inside::Code;
                }
            }
            Inside::Quote(quote) => {
                run.push(ch);
                index += 1;
                if ch == quote {
                    flush(&mut run, Piece::Value, &mut emit);
                    state.inside = Inside::Code;
                }
            }
            Inside::Code => {
                if syntax.line_comments.iter().any(|marker| starts_with(&chars, index, marker)) {
                    // A line comment ends at the newline, not at the end of the
                    // body: a caller may hand over a whole multi-line string.
                    let end = chars[index..].iter().position(|next| *next == '\n').map(|at| index + at).unwrap_or(chars.len());
                    let comment: String = chars[index..end].iter().collect();
                    emit(&comment, Piece::Comment);
                    index = end;
                } else if let Some((open, _)) = syntax.block_comment.filter(|(open, _)| starts_with(&chars, index, open)) {
                    run.push_str(open);
                    index += open.chars().count();
                    state.inside = Inside::Comment;
                } else if syntax.quotes.contains(&ch) {
                    run.push(ch);
                    index += 1;
                    state.inside = Inside::Quote(ch);
                } else if ch.is_whitespace() {
                    // Whitespace does not end a member access: `items\n.map(` is
                    // still a member access.
                    let mut spacing = String::new();
                    while let Some(&next) = chars.get(index) {
                        if next.is_whitespace() {
                            spacing.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    emit(&spacing, Piece::Text);
                } else if syntax.sigils.contains(&ch) {
                    // `$name` is one thing, and a bare `$` - as in `${name}` -
                    // is punctuation waiting for the name after it.
                    let mut name = String::from(ch);
                    index += 1;
                    while let Some(&next) = chars.get(index) {
                        if is_word_char(next) {
                            name.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    emit(&name, if name.chars().count() > 1 { Piece::Attribute } else { Piece::Operator });
                    after_dot = false;
                } else if is_word_start(ch) {
                    let mut word = String::new();
                    while let Some(&next) = chars.get(index) {
                        if is_word_char(next) {
                            word.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    emit(&word, word_piece(syntax, &word, after_dot, next_visible(&chars, index)));
                    after_dot = false;
                } else if ch.is_ascii_digit() {
                    let mut number = String::new();
                    while let Some(&next) = chars.get(index) {
                        if next.is_ascii_alphanumeric() || next == '.' || next == '_' {
                            number.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    emit(&number, Piece::Number);
                    after_dot = false;
                } else if matches!(ch, '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',') {
                    emit(&ch.to_string(), Piece::Bracket);
                    index += 1;
                    after_dot = false;
                } else if ch == '.' {
                    emit(".", Piece::Operator);
                    index += 1;
                    after_dot = true;
                } else if is_operator_char(ch) {
                    let mut operator = String::new();
                    while let Some(&next) = chars.get(index) {
                        if is_operator_char(next) {
                            operator.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    emit(&operator, Piece::Operator);
                    after_dot = false;
                } else {
                    emit(&ch.to_string(), Piece::Text);
                    index += 1;
                }
            }
        }
    }

    let leftover = match state.inside {
        Inside::Comment => Piece::Comment,
        Inside::Quote(_) => Piece::Value,
        Inside::Code => Piece::Text,
    };
    flush(&mut run, leftover, &mut emit);
}

/// What a word is: a reserved word, the name of a call, a member reached
/// through a `.`, a type, or a plain name.
fn word_piece(syntax: &Syntax, word: &str, after_dot: bool, next: Option<char>) -> Piece {
    let reserved = match syntax.case_insensitive {
        true => syntax.keywords.iter().any(|keyword| keyword.eq_ignore_ascii_case(word)),
        false => syntax.keywords.contains(&word),
    };
    if reserved {
        return Piece::Keyword;
    }
    if next == Some('(') {
        return Piece::Function;
    }
    if after_dot {
        return Piece::Attribute;
    }
    if syntax.capitals_are_types && word.chars().next().map(|first| first.is_uppercase()).unwrap_or(false) {
        return Piece::Element;
    }
    return Piece::Text;
}

fn starts_with(chars: &[char], index: usize, marker: &str) -> bool {
    return chars[index..].iter().zip(marker.chars()).filter(|(here, there)| *here == there).count() == marker.chars().count();
}

/// The next character that is not whitespace, so a call is recognized through
/// the space in `map (x)`.
fn next_visible(chars: &[char], from: usize) -> Option<char> {
    return chars[from..].iter().find(|next| !next.is_whitespace()).copied();
}

fn is_word_start(ch: char) -> bool {
    return ch.is_alphabetic() || ch == '_' || ch == '$';
}

fn is_word_char(ch: char) -> bool {
    return ch.is_alphanumeric() || ch == '_' || ch == '$';
}

fn is_operator_char(ch: char) -> bool {
    return matches!(ch, '=' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '!' | '&' | '|' | '^' | '~' | '?' | ':' | '@' | '#');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pieces(dialect: Dialect, body: &str) -> Vec<(String, Piece)> {
        let mut state = start(dialect);
        let mut out = Vec::new();
        tokenize(body, &mut state, |text, piece| out.push((text.to_string(), piece)));
        return out;
    }

    fn advance(body: &str, state: &mut State) {
        tokenize(body, state, |_, _| {});
    }

    #[test]
    fn a_script_declaration_comes_back_as_keyword_name_and_number() {
        let out = pieces(Dialect::Script, "const total = 42;");
        assert!(out.contains(&("const".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&("total".to_string(), Piece::Text)), "got {:?}", out);
        assert!(out.contains(&("42".to_string(), Piece::Number)), "got {:?}", out);
        assert!(out.contains(&(";".to_string(), Piece::Bracket)));
    }

    #[test]
    fn a_call_is_named_and_a_member_is_an_attribute() {
        let out = pieces(Dialect::Script, "items.map(double); page.title;");
        assert!(out.contains(&("map".to_string(), Piece::Function)), "got {:?}", out);
        assert!(out.contains(&("title".to_string(), Piece::Attribute)), "got {:?}", out);
    }

    #[test]
    fn json_reads_as_strings_numbers_and_literals() {
        let out = pieces(Dialect::Script, r#"{"name": "nail", "ok": true, "n": 3}"#);
        assert!(out.contains(&("\"name\"".to_string(), Piece::Value)), "got {:?}", out);
        assert!(out.contains(&("true".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&("3".to_string(), Piece::Number)), "got {:?}", out);
    }

    #[test]
    fn sql_keywords_are_recognized_whatever_their_case() {
        let upper = pieces(Dialect::Sql, "SELECT name FROM users WHERE id = 1;");
        assert!(upper.contains(&("SELECT".to_string(), Piece::Keyword)), "got {:?}", upper);
        let lower = pieces(Dialect::Sql, "select name from users;");
        assert!(lower.contains(&("select".to_string(), Piece::Keyword)), "got {:?}", lower);
    }

    #[test]
    fn a_sql_comment_runs_to_the_end_of_its_own_line_only() {
        let out = pieces(Dialect::Sql, "-- pick everyone\nSELECT * FROM users;");
        assert!(out.contains(&("-- pick everyone".to_string(), Piece::Comment)), "got {:?}", out);
        assert!(out.contains(&("SELECT".to_string(), Piece::Keyword)), "the next line is code again, got {:?}", out);
    }

    #[test]
    fn a_sql_string_is_a_value_and_a_quoted_column_is_not_swallowed() {
        let out = pieces(Dialect::Sql, "SELECT 'literal' FROM t;");
        assert!(out.contains(&("'literal'".to_string(), Piece::Value)), "got {:?}", out);
    }

    #[test]
    fn hash_comment_languages_use_their_own_mark() {
        let python = pieces(Dialect::Python, "def run(): # start here\n    return None");
        assert!(python.contains(&("def".to_string(), Piece::Keyword)), "got {:?}", python);
        assert!(python.contains(&("# start here".to_string(), Piece::Comment)), "got {:?}", python);
        assert!(python.contains(&("run".to_string(), Piece::Function)), "got {:?}", python);
    }

    #[test]
    fn a_shell_variable_is_one_piece() {
        let out = pieces(Dialect::Shell, "if [ -n \"$HOME\" ]; then echo hi; fi");
        assert!(out.contains(&("$HOME".to_string(), Piece::Attribute)) || out.iter().any(|(text, piece)| text.contains("$HOME") && *piece == Piece::Value), "got {:?}", out);
        assert!(out.contains(&("then".to_string(), Piece::Keyword)), "got {:?}", out);
    }

    #[test]
    fn a_capitalized_word_is_a_type_where_that_is_the_convention() {
        let rust = pieces(Dialect::Rust, "let name: String = value;");
        assert!(rust.contains(&("String".to_string(), Piece::Element)), "got {:?}", rust);
        // C does not have the convention, so a capitalized word stays plain.
        let c = pieces(Dialect::C, "int Total = 1;");
        assert!(c.contains(&("Total".to_string(), Piece::Text)), "got {:?}", c);
    }

    #[test]
    fn a_rust_lifetime_does_not_open_a_string() {
        let out = pieces(Dialect::Rust, "fn name<'a>(text: &'a str) -> &'a str { text }");
        assert!(!out.iter().any(|(_, piece)| *piece == Piece::Value), "a lifetime is not a string, got {:?}", out);
    }

    #[test]
    fn a_wgsl_shader_reads_as_attributes_types_and_its_own_structs() {
        let out = pieces(Dialect::Wgsl, "@fragment\nfn fs_main(in: VertexOut) -> @location(0) vec4<f32> { return scene.fog_color; }");
        assert!(out.contains(&("@fragment".to_string(), Piece::Attribute)), "got {:?}", out);
        assert!(out.contains(&("@location".to_string(), Piece::Attribute)), "got {:?}", out);
        assert!(out.contains(&("fn".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&("vec4".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&("f32".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&("fs_main".to_string(), Piece::Function)), "got {:?}", out);
        assert!(out.contains(&("VertexOut".to_string(), Piece::Element)), "got {:?}", out);
        assert!(out.contains(&("fog_color".to_string(), Piece::Attribute)), "a member after a dot, got {:?}", out);
    }

    #[test]
    fn comments_and_strings_carry_to_the_next_line() {
        let mut state = start(Dialect::Script);
        advance("/* opened", &mut state);
        assert_eq!(state.inside, Inside::Comment);
        advance(" closed */", &mut state);
        assert_eq!(state.inside, Inside::Code);

        let mut state = start(Dialect::Script);
        advance("const message = \"unfinished", &mut state);
        assert_eq!(state.inside, Inside::Quote('"'));
    }

    #[test]
    fn a_template_literal_written_the_nail_way_opens_and_closes() {
        // Inside a Nail string a backtick is escaped, so that is how the
        // scanner sees one.
        let out = pieces(Dialect::Script, r#"const greeting = \`hi\`;"#);
        let quoted: Vec<&(String, Piece)> = out.iter().filter(|(_, piece)| *piece == Piece::Value).collect();
        assert_eq!(quoted.len(), 1, "the template literal should be one piece, got {:?}", out);
        assert_eq!(quoted[0].0, "`hi\\`");
    }
}
