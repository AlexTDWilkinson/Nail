/// Counts backticks that are not escaped with a backslash. Used to track
/// whether a line opens or closes a multi-line string literal.
fn count_unescaped_backticks(line: &str) -> usize {
    let mut count = 0;
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            chars.next();
        } else if ch == '`' {
            count += 1;
        }
    }
    count
}

pub fn format_nail_code(lines: &[String]) -> Vec<String> {
    let mut formatted_lines = Vec::new();
    let mut indent_level: usize = 0;
    let mut last_line_had_closing_brace = false;
    let mut in_multiline_string = false;

    // A shebang and a version line are addressed to the launcher rather than to
    // the language, so they pass through untouched. Reading them as code would
    // reformat them into something the launcher can no longer parse, which would
    // silently unpin the file.
    let header_lines = crate::version_line::scan_header(lines.join("\n").as_bytes()).lines as usize;

    for (i, line) in lines.iter().enumerate() {
        if i < header_lines {
            formatted_lines.push(line.clone());
            last_line_had_closing_brace = false;
            continue;
        }

        // Lines that are part of a multi-line string literal (or that open one)
        // must pass through verbatim: reformatting or re-indenting them would
        // change the string's contents.
        if in_multiline_string {
            if count_unescaped_backticks(line) % 2 == 1 {
                in_multiline_string = false;
            }
            formatted_lines.push(line.clone());
            last_line_had_closing_brace = false;
            continue;
        }
        if count_unescaped_backticks(line) % 2 == 1 {
            in_multiline_string = true;
            formatted_lines.push(line.clone());
            last_line_had_closing_brace = false;
            continue;
        }

        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            formatted_lines.push(String::new());
            last_line_had_closing_brace = false;
            continue;
        }

        // Check if this line has content followed by a closing brace (like "age:i}" or "age:i }")
        // Split it into two separate lines
        // Lines containing string literals are never split: the brace may be inside the string.
        // Only the code part of the line is considered: a brace inside a
        // trailing comment is text, and splitting the line there cut the
        // comment in half and left a file that no longer lexes.
        let code = code_before_comment(trimmed);
        let needs_split = (code.contains("}") && !code.starts_with("}") &&
                          !code.contains("{") && !code.contains("->") &&
                          !code.contains('`') &&
                          !trimmed.starts_with("//")) &&
                          code.chars().filter(|&c| c != '}' && c != ' ').count() > 0;
        
        if needs_split {
            // Split the line at the closing brace
            if let Some(brace_pos) = code.rfind('}') {
                let content_part = &trimmed[..brace_pos];
                let brace_part = &trimmed[brace_pos..];
                
                // Process the content part
                if !content_part.trim().is_empty() {
                    let formatted_content = format_nail_line(content_part.trim());
                    let indented = format!("{}{}", "    ".repeat(indent_level), formatted_content);
                    formatted_lines.push(indented);
                }
                
                // Process the closing brace(s) on a new line
                // Decrease indent BEFORE formatting the brace line
                let brace_count = brace_part.matches('}').count();
                indent_level = indent_level.saturating_sub(brace_count);
                let brace_formatted = format_nail_line(brace_part.trim());
                let brace_indented = format!("{}{}", "    ".repeat(indent_level), brace_formatted);
                formatted_lines.push(brace_indented);
                
                last_line_had_closing_brace = indent_level == 0;
                continue;
            }
        }

        // Check if this line starts a new block (function, struct, enum, etc.)
        let starts_new_top_level_block = trimmed.starts_with("f ") || trimmed.starts_with("struct ") || trimmed.starts_with("enum ") || trimmed.starts_with("parallel ");

        // Only consider 'if' as starting a new block if it's at the top level
        let starts_new_block = starts_new_top_level_block || (trimmed.starts_with("if ") && indent_level == 0);

        // Add blank line before new blocks (except at the beginning or after comments)
        if starts_new_block && i > 0 && !formatted_lines.is_empty() {
            let last_non_empty_idx = formatted_lines.iter().rposition(|l| !l.trim().is_empty());
            if let Some(idx) = last_non_empty_idx {
                let last_line = formatted_lines[idx].trim();
                // Add blank line if:
                // - Previous line ends with } or ; (end of block/statement)
                // - Previous line is a single-line function
                // - Not after a comment section
                if (last_line.ends_with('}') || last_line.ends_with(';') || (last_line.starts_with("f ") && last_line.contains('{'))) && !last_line.starts_with("//") {
                    // Check if there's already a blank line
                    if formatted_lines.last().map_or(true, |l| !l.trim().is_empty()) {
                        formatted_lines.push(String::new());
                    }
                }
            }
        }

        // Add blank line after closing brace at top level
        if last_line_had_closing_brace && !trimmed.is_empty() && !trimmed.starts_with("//") {
            // Don't add blank line if the previous line already added one
            if formatted_lines.last().map_or(true, |l| !l.trim().is_empty()) {
                formatted_lines.push(String::new());
            }
        }

        // Decrease indent for closing braces
        if trimmed.starts_with('}') || trimmed.starts_with(']') {
            indent_level = indent_level.saturating_sub(1);
        }

        // Format the line content
        let formatted_content = format_nail_line(trimmed);

        // Apply indentation
        let indented = format!("{}{}", "    ".repeat(indent_level), formatted_content);

        formatted_lines.push(indented);

        // Track whether this line closed a top level block, which is what a
        // blank line separates. The line has to be the brace itself: a
        // one-line `if { ... }` also ends with a brace, and treating it as
        // the end of a block meant formatting inserted another blank line
        // every time it ran, so the file never settled.
        last_line_had_closing_brace = (trimmed == "}" || formatted_content.trim() == "}") && indent_level == 0;

        // Increase indent after opening braces
        if trimmed.ends_with('{') || formatted_content.ends_with('{') {
            indent_level += 1;
        }

        // Handle special cases like "else" on same line as "}"
        if trimmed.contains("} else {") || formatted_content.contains("} else {") {
            // Don't change indent
        } else if trimmed.contains("},") || formatted_content.contains("},") {
            // For lines like "}, " in enums/structs
        }
    }

    formatted_lines
}

/// Write an operator with one space on each side, unless the word in front of
/// it would stop being that word.
///
/// Both halves matter. Trimming first is what makes formatting an already
/// formatted line a no-op: writing " != " onto text that already ends in a
/// space added another one on every pass. And `y`, `r`, `p` and `c` are
/// keywords only when whitespace follows them, so `y+ 3` adds to a variable
/// named y while `y + 3` yields 3. Those are different programs, and a
/// formatter may not turn one into the other.
fn push_spaced_operator(formatted: &mut String, operator: &str) {
    if spacing_changes_the_word(formatted) {
        formatted.push_str(operator);
        formatted.push(' ');
        return;
    }
    while formatted.ends_with(' ') {
        formatted.pop();
    }
    formatted.push(' ');
    formatted.push_str(operator);
    formatted.push(' ');
}

/// Whether a space between the word just written and an operator would change
/// what that word means. The lexer answers, by being asked the same question
/// twice: once with the space and once without.
fn spacing_changes_the_word(formatted: &str) -> bool {
    let word: String = formatted.chars().rev().take_while(|character| character.is_alphanumeric() || *character == '_').collect::<Vec<char>>().into_iter().rev().collect();
    if word.is_empty() {
        return false;
    }
    let kind_of = |text: String| crate::lexer::lexer_without_imports(&text).first().map(|token| std::mem::discriminant(&token.token_type));
    kind_of(format!("{} +", word)) != kind_of(format!("{}+", word))
}

/// The part of a line that is code, with any trailing comment cut off. A
/// brace inside a comment is text rather than structure, and the difference
/// decides whether a line is split in two.
fn code_before_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut index = 0;
    while index < bytes.len() {
        // Only ASCII is examined, and every byte of a multi-byte character
        // has its high bit set, so none of them can be mistaken for one of
        // these markers.
        if bytes[index] == b'`' && (index == 0 || bytes[index - 1] != b'\\') {
            in_string = !in_string;
        } else if !in_string && bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            return &line[..index];
        }
        index += 1;
    }
    line
}

/// Whether a '/' starts `/p` or `/c`, the tokens that close a parallel or a
/// concurrent block. Each is one token and must not be spaced out the way
/// division is. A name can follow a division sign with no space between them
/// (`10 /price_total`), so the letter only closes a block when nothing that
/// could continue a name comes after it.
fn closes_a_block(chars: &std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    match lookahead.next() {
        Some('p') | Some('c') => !lookahead.next().map_or(false, |next| next.is_alphanumeric() || next == '_'),
        _ => false,
    }
}

/// Whether the '<' about to be written opens a hashmap type, `h<s,i>`, which
/// is the only place in Nail where '<' is a bracket rather than a comparison.
/// The letter has to stand alone to count: a name that happens to end in h,
/// as in `path < limit`, is a comparison like any other.
fn opens_a_hashmap_type(formatted: &str) -> bool {
    let trimmed = formatted.trim_end();
    let mut characters = trimmed.chars().rev();
    if characters.next() != Some('h') {
        return false;
    }
    !characters.next().map_or(false, |before| before.is_alphanumeric() || before == '_')
}

/// Whether what has been written so far ends in a name, as opposed to a
/// keyword, a number or a symbol. The lexer decides, rather than a list of
/// keywords kept here: `r`, `p`, `c` and `y` are keywords only in some
/// positions, and one statement of that rule in the language is enough.
fn preceding_word_is_a_name(formatted: &str) -> bool {
    let word: String = formatted.trim_end().chars().rev().take_while(|character| character.is_alphanumeric() || *character == '_').collect::<Vec<char>>().into_iter().rev().collect();
    if word.is_empty() {
        return false;
    }
    // The word is lexed with the '(' that is about to follow it, because that
    // is exactly the context the lexer uses to tell `y (x)` from `y(x)`.
    let probe = format!("{} (", word);
    matches!(crate::lexer::lexer_without_imports(&probe).first().map(|token| &token.token_type), Some(crate::lexer::TokenType::Identifier(_)))
}

pub fn format_nail_line(line: &str) -> String {
    // Skip empty lines
    if line.trim().is_empty() {
        return String::new();
    }

    // Skip comment lines (don't format them)
    if line.trim().starts_with("//") {
        return line.to_string();
    }

    let mut formatted = String::new();
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut in_comment = false;
    let mut brace_stack: Vec<bool> = Vec::new();  // Track whether each brace level is a struct init
    let mut angle_depth: usize = 0; // Track unmatched '<' so generic types like h<s,s> stay untouched

    while let Some(ch) = chars.next() {
        // Check for string start/end
        if ch == '`' && !in_comment {
            in_string = !in_string;
            formatted.push(ch);
            continue;
        }

        // Check for comment start
        if ch == '/' && chars.peek() == Some(&'/') && !in_string {
            in_comment = true;
            // Ensure space before comment if not at start of line
            if !formatted.is_empty() && !formatted.ends_with(' ') {
                formatted.push(' ');
            }
            formatted.push(ch);
            formatted.push(chars.next().unwrap());
            // Add space after // for readability
            if chars.peek().is_some() && chars.peek() != Some(&' ') {
                formatted.push(' ');
            }
            continue;
        }

        // If in string or comment, don't format
        if in_string || in_comment {
            formatted.push(ch);
            continue;
        }

        // Track brace context for struct initialization
        if ch == '{' {
            // Check if this is a struct initialization by looking back
            // Struct init pattern: CapitalizedName { or , { (for nested structs)
            let is_struct = {
                let trimmed = formatted.trim_end();
                // Check if preceded by a capitalized identifier (struct name)
                if let Some(last_word_start) = trimmed.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                    let last_word = &trimmed[last_word_start + 1..];
                    last_word.chars().next().map_or(false, |c| c.is_ascii_uppercase())
                } else {
                    // If no non-alphanumeric found, check the whole string
                    trimmed.chars().next().map_or(false, |c| c.is_ascii_uppercase())
                }
            };
            brace_stack.push(is_struct);
        } else if ch == '}' {
            brace_stack.pop();
        }

        // Format operators
        match ch {
            '=' => {
                // Check if we're inside a struct initialization
                let in_struct_init = brace_stack.last().copied().unwrap_or(false);

                // Trim trailing space before operator
                while formatted.ends_with(' ') {
                    formatted.pop();
                }

                if chars.peek() == Some(&'=') {
                    // ==
                    push_spaced_operator(&mut formatted, "==");
                    chars.next();
                } else if in_struct_init {
                    // = in struct initialization - no space before, one space after
                    formatted.push_str("= ");
                    // Skip any following spaces (we already added one)
                    while chars.peek() == Some(&' ') {
                        chars.next();
                    }
                } else {
                    // = regular assignment
                    push_spaced_operator(&mut formatted, "=");
                }
            }
            '!' => {
                if chars.peek() == Some(&'=') {
                    // !=
                    push_spaced_operator(&mut formatted, "!=");
                    chars.next();
                } else {
                    // Check if this is an error type (e.g., i!e, f!e, s!e)
                    // Look back to see if we just had a type character
                    let last_char = formatted.chars().last();
                    let is_type_char = last_char.map_or(false, |c| matches!(c, 'i' | 'f' | 's' | 'b' | 'a'));

                    // Look ahead to see if next char is 'e' (error)
                    let next_is_e = chars.peek() == Some(&'e');

                    if is_type_char && next_is_e {
                        // This is an error type like i!e, don't add spaces
                        formatted.push(ch);
                    } else {
                        // Regular ! operator
                        formatted.push(ch);
                    }
                }
            }
            '<' => {
                if chars.peek() == Some(&'=') {
                    // <=
                    push_spaced_operator(&mut formatted, "<=");
                    chars.next();
                } else {
                    // A '<' opens a type only in a hashmap type, h<s,s>.
                    // Everywhere else it is less-than, and counting it as an
                    // open bracket made the next '>=' on the line read as a
                    // closing bracket followed by an assignment, which turned
                    // 'count >= 1' into 'count > = 1'.
                    if opens_a_hashmap_type(&formatted) {
                        angle_depth += 1;
                    }
                    formatted.push(ch);
                }
            }
            '>' => {
                if angle_depth > 0 {
                    // Closing a generic type parameter list like h<s,s>;
                    // a following '=' belongs to an assignment, not '>='.
                    angle_depth -= 1;
                    formatted.push(ch);
                } else if chars.peek() == Some(&'=') {
                    // >=
                    push_spaced_operator(&mut formatted, ">=");
                    chars.next();
                } else {
                    // Bare '>' comparison: preserve as written.
                    formatted.push(ch);
                }
            }
            '+' | '-' | '*' | '%' => {
                if ch == '-' && chars.peek() == Some(&'>') {
                    // ->
                    while formatted.ends_with(' ') {
                        formatted.pop();
                    }
                    formatted.push_str(" -> ");
                    chars.next();
                } else if ch == '-' && (formatted.chars().last().map_or(true, |c| !c.is_alphanumeric() && c != ')') || !preceding_word_is_a_name(&formatted)) {
                    // A minus sign rather than subtraction: nothing to
                    // subtract from, or what comes before it is a keyword
                    // rather than a value. `scan ... from-1 {` starts from
                    // minus one, and spacing it into `from - 1` made it read
                    // as a subtraction and changed the program.
                    formatted.push(ch);
                } else {
                    push_spaced_operator(&mut formatted, &ch.to_string());
                }
            }
            '/' => {
                // Check if this is part of a comment
                if chars.peek() == Some(&'/') {
                    // This is handled by the comment check above, but just in case
                    formatted.push(ch);
                } else if closes_a_block(&chars) {
                    // This is /p or /c, the end of a parallel or concurrent
                    // block. Both are one token, and spacing one like
                    // division leaves a file that no longer lexes.
                    formatted.push(ch);
                    formatted.push(chars.next().unwrap()); // consume the 'p' or the 'c'
                } else {
                    // Regular division operator
                    push_spaced_operator(&mut formatted, "/");
                }
            }
            ',' => {
                // Remove space before comma
                while formatted.ends_with(' ') {
                    formatted.pop();
                }
                formatted.push(',');
                formatted.push(' ');
            }
            ';' => {
                // Remove space before semicolon, no space after (unless followed by comment)
                while formatted.ends_with(' ') {
                    formatted.pop();
                }
                formatted.push(ch);
            }
            '(' => {
                // A '(' straight after a name opens that name's arguments, so
                // `print (message)` closes up into `print(message)`. After
                // anything else the space is load bearing: `y (count + 1)`
                // yields a value and `y(count + 1)` calls a function named y,
                // which is a different program and usually not one that
                // compiles.
                if formatted.ends_with(' ') && preceding_word_is_a_name(&formatted) {
                    while formatted.ends_with(' ') {
                        formatted.pop();
                    }
                }
                formatted.push(ch);
            }
            ' ' => {
                // Only add space if the last char wasn't already a space
                if !formatted.ends_with(' ') {
                    formatted.push(ch);
                }
            }
            _ => formatted.push(ch),
        }
    }

    // Note: no blanket double-space collapsing here — it would mangle the
    // contents of string literals and comments. Runs of spaces outside
    // strings are already collapsed by the ' ' arm above.
    formatted.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operators() {
        assert_eq!(format_nail_line("x=5"), "x = 5");
        assert_eq!(format_nail_line("a+b"), "a + b");
        // A name rather than `c`, which opens a concurrent block: a minus
        // after a keyword belongs to the number after it, not to a
        // subtraction, and single letters are not names in Nail anyway.
        assert_eq!(format_nail_line("count-depth"), "count - depth");
        assert_eq!(format_nail_line("e*f"), "e * f");
        assert_eq!(format_nail_line("g/h"), "g / h");
    }

    #[test]
    fn test_comparison_operators() {
        assert_eq!(format_nail_line("a==b"), "a == b");
        assert_eq!(format_nail_line("c!=d"), "c != d");
        assert_eq!(format_nail_line("e<=f"), "e <= f");
        assert_eq!(format_nail_line("g>=h"), "g >= h");
        // Bare < and > are ambiguous with generic types (h<s,s>), so they are
        // preserved exactly as written rather than force-spaced.
        assert_eq!(format_nail_line("i<j"), "i<j");
        assert_eq!(format_nail_line("k>l"), "k>l");
        assert_eq!(format_nail_line("a < b"), "a < b");
        assert_eq!(format_nail_line("x > y"), "x > y");
    }

    #[test]
    fn test_hashmap_generic_types() {
        assert_eq!(format_nail_line("map1:h<s,s> = hashmap_new()"), "map1:h<s, s> = hashmap_new()");
        assert_eq!(format_nail_line("map1:h<s,s>=hashmap_new()"), "map1:h<s, s> = hashmap_new()");
        assert_eq!(format_nail_line("map2:h<s,i> = hashmap_new()"), "map2:h<s, i> = hashmap_new()");
    }

    #[test]
    fn test_string_contents_preserved() {
        // Runs of spaces inside string literals must survive formatting
        assert_eq!(format_nail_line("x:s = `hello  world`;"), "x:s = `hello  world`;");
        assert_eq!(format_nail_line("y:s = `a<b  and  c>d`;"), "y:s = `a<b  and  c>d`;");
    }

    #[test]
    fn test_multiline_strings_pass_through() {
        let input = vec![
            "text:s = `first line".to_string(),
            "  indented   string  content".to_string(),
            "last line`;".to_string(),
            "x=1;".to_string(),
        ];
        let formatted = format_nail_code(&input);
        assert_eq!(formatted[0], "text:s = `first line");
        assert_eq!(formatted[1], "  indented   string  content");
        assert_eq!(formatted[2], "last line`;");
        assert_eq!(formatted[3], "x = 1;");
    }

    #[test]
    fn test_arrow_operator() {
        assert_eq!(format_nail_line("if x -> y"), "if x -> y");
        assert_eq!(format_nail_line("case->result"), "case -> result");
    }

    #[test]
    fn test_function_calls() {
        assert_eq!(format_nail_line("print (x)"), "print(x)");
        assert_eq!(format_nail_line("f greet (name)"), "f greet(name)");
        assert_eq!(format_nail_line("safe(divide(10, 2),msg)"), "safe(divide(10, 2), msg)");
    }

    #[test]
    fn test_comma_spacing() {
        assert_eq!(format_nail_line("a,b,c"), "a, b, c");
        assert_eq!(format_nail_line("func(x,y,z)"), "func(x, y, z)");
    }

    #[test]
    fn test_preserve_comments() {
        assert_eq!(format_nail_line("// This is a comment"), "// This is a comment");
        assert_eq!(format_nail_line("x = 5 // inline comment"), "x = 5 // inline comment");
        assert_eq!(format_nail_line("x = 5// inline comment"), "x = 5 // inline comment");
        assert_eq!(format_nail_line("x = 5//inline comment"), "x = 5 // inline comment");
        assert_eq!(format_nail_line("final_message:s = `Nail!`;//Inline comment"), "final_message:s = `Nail!`; // Inline comment");
    }

    #[test]
    fn test_preserve_strings() {
        assert_eq!(format_nail_line("`hello world`"), "`hello world`");
        assert_eq!(format_nail_line("s = `test + string`"), "s = `test + string`");
    }

    #[test]
    fn test_error_types() {
        assert_eq!(format_nail_line("f div():i!e"), "f div():i!e");
        assert_eq!(format_nail_line("result:i!e = divide(a,b)"), "result:i!e = divide(a, b)");
        assert_eq!(format_nail_line("f divide(num:i, den:i):i!e {"), "f divide(num:i, den:i):i!e {");
        assert_eq!(format_nail_line("f safe(result:i!e, handler:s):i {"), "f safe(result:i!e, handler:s):i {");
        assert_eq!(format_nail_line("value:f!e = parse_float(str)"), "value:f!e = parse_float(str)");
        assert_eq!(format_nail_line("data:s!e = read_file(path)"), "data:s!e = read_file(path)");
    }

    #[test]
    fn test_negative_numbers() {
        assert_eq!(format_nail_line("x = -5"), "x = -5");
        // 'y' keeps its operator glued to it, because a space after it turns
        // it into the yield keyword. Any other name spaces out as usual.
        assert_eq!(format_nail_line("y = a - -b"), "y= a - -b");
        assert_eq!(format_nail_line("y_value = a - -b"), "y_value = a - -b");
        assert_eq!(format_nail_line("z = -10 + 5"), "z = -10 + 5");
    }

    #[test]
    fn test_multiple_spaces() {
        assert_eq!(format_nail_line("x    =    5"), "x = 5");
        assert_eq!(format_nail_line("a  +  b  *  c"), "a + b * c");
    }

    #[test]
    fn test_type_annotations() {
        assert_eq!(format_nail_line("name:s = `Alice`"), "name:s = `Alice`");
        assert_eq!(format_nail_line("numbers:a:i = [1,2,3]"), "numbers:a:i = [1, 2, 3]");
        assert_eq!(format_nail_line("result:i = calc()"), "result:i = calc()");
    }

    #[test]
    fn test_struct_initialization() {
        assert_eq!(format_nail_line("Person { age = 0 }"), "Person { age= 0 }");
        assert_eq!(format_nail_line("Person { name = `Alice`, age = 30 }"), "Person { name= `Alice`, age= 30 }");
        assert_eq!(format_nail_line("Point { x_coord = 10, y_coord = 20 }"), "Point { x_coord= 10, y_coord= 20 }");
        // Regular assignment should still have spaces
        assert_eq!(format_nail_line("person:Person = Person { age = 25 }"), "person:Person = Person { age= 25 }");
    }

    #[test]
    fn test_complex_expression() {
        assert_eq!(format_nail_line("sum_squares:i = reduce(map(nums, square_func),0,add_func)"), "sum_squares:i = reduce(map(nums, square_func), 0, add_func)");
    }

    #[test]
    fn test_empty_and_whitespace_lines() {
        assert_eq!(format_nail_line(""), "");
        assert_eq!(format_nail_line("   "), "");
        assert_eq!(format_nail_line("\t"), "");
    }

    #[test]
    fn test_code_indentation() {
        let input = vec!["f greet(name:s):s {".to_string(), "parts:a:s = [`Hello, `, name, `!`];".to_string(), "r array_join(parts);".to_string(), "}".to_string()];

        let expected = vec!["f greet(name:s):s {".to_string(), "    parts:a:s = [`Hello, `, name, `!`];".to_string(), "    r array_join(parts);".to_string(), "}".to_string()];

        assert_eq!(format_nail_code(&input), expected);
    }

    #[test]
    fn test_nested_indentation() {
        let input = vec![
            "if {".to_string(),
            "x > 0 -> {".to_string(),
            "print(`positive`);".to_string(),
            "},".to_string(),
            "else -> {".to_string(),
            "print(`negative`);".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];

        let expected = vec![
            "if {".to_string(),
            "    x > 0 -> {".to_string(),
            "        print(`positive`);".to_string(),
            "    },".to_string(),
            "    else -> {".to_string(),
            "        print(`negative`);".to_string(),
            "    }".to_string(),
            "}".to_string(),
        ];

        assert_eq!(format_nail_code(&input), expected);
    }

    #[test]
    fn test_function_spacing() {
        let input = vec![
            "f double_func(num:i):i { r num * 2; }".to_string(),
            "f is_even_func(n:i):b {".to_string(),
            "r n % 2 == 0;".to_string(),
            "}".to_string(),
            "f add_func(acc:i, n:i):i { r acc + n; }".to_string(),
        ];

        let expected = vec![
            "f double_func(num:i):i { r num * 2; }".to_string(),
            "".to_string(),
            "f is_even_func(n:i):b {".to_string(),
            "    r n % 2 == 0;".to_string(),
            "}".to_string(),
            "".to_string(),
            "f add_func(acc:i, n:i):i { r acc + n; }".to_string(),
        ];

        assert_eq!(format_nail_code(&input), expected);
    }

    #[test]
    fn test_real_files_survive_formatting() {
        // Formatting must never break valid programs: the formatted output has
        // to lex without errors, keep every string literal byte-identical, and
        // still parse. Formatting must also be idempotent.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let files = ["tests/test_hashmap.nail", "tests/test_arrays.nail", "tests/test_tagged_strings.nail", "examples/hello_world.nail"];
        let mut files_verified = 0;

        for file in files {
            let path = format!("{}/{}", manifest_dir, file);
            let source = match std::fs::read_to_string(&path) {
                Ok(source) => source,
                Err(_) => continue, // File layout changed; nothing to verify
            };
            let lines: Vec<String> = source.lines().map(String::from).collect();

            let original_tokens = crate::lexer::lexer(&source);
            let original_had_lex_errors = original_tokens.iter().any(|t| matches!(t.token_type, crate::lexer::TokenType::LexerError(_)));
            let original_parsed = crate::parser::parse(original_tokens.clone()).is_ok();
            if original_had_lex_errors || !original_parsed {
                continue; // Only valid programs are meaningful inputs here
            }

            let formatted = format_nail_code(&lines);
            let formatted_source = formatted.join("\n");
            let formatted_tokens = crate::lexer::lexer(&formatted_source);

            for token in &formatted_tokens {
                if let crate::lexer::TokenType::LexerError(message) = &token.token_type {
                    panic!("Formatting {} introduced lexer error: {}", file, message);
                }
            }

            let string_literals = |tokens: &[crate::lexer::Token]| -> Vec<String> {
                tokens
                    .iter()
                    .filter_map(|t| if let crate::lexer::TokenType::StringLiteral { value, .. } = &t.token_type { Some(value.clone()) } else { None })
                    .collect()
            };
            assert_eq!(string_literals(&original_tokens), string_literals(&formatted_tokens), "Formatting {} changed string literal contents", file);

            assert!(crate::parser::parse(formatted_tokens).is_ok(), "Formatting {} broke parsing", file);

            let reformatted = format_nail_code(&formatted);
            assert_eq!(reformatted, formatted, "Formatting {} is not idempotent", file);
            files_verified += 1;
        }

        assert!(files_verified > 0, "No valid .nail files were available to verify formatting against");
    }

    #[test]
    fn test_parallel_syntax() {
        assert_eq!(format_nail_line("p"), "p");
        assert_eq!(format_nail_line("/p"), "/p");
        assert_eq!(format_nail_line("task1:s = `hello`; /p"), "task1:s = `hello`; /p");
        assert_eq!(format_nail_line("p task1:i = 42; task2:s = `test`; /p"), "p task1:i = 42; task2:s = `test`; /p");
    }

    /// The fuzzer found each of these by formatting a program that compiled
    /// and finding one that no longer did.
    #[test]
    fn test_formatting_never_changes_meaning() {
        // A concurrent block ends with /c, which is one token. Spacing it
        // like division left a file that no longer lexes.
        assert_eq!(format_nail_line("/c"), "/c");
        assert_eq!(format_nail_line("c label_one:s = `x`; /c"), "c label_one:s = `x`; /c");
        // Division is still division, whatever letter follows it.
        assert_eq!(format_nail_line("half_value:i = total_value / 2;"), "half_value:i = total_value / 2;");
        assert_eq!(format_nail_line("share_value:i = total_value /count_value;"), "share_value:i = total_value / count_value;");

        // '<' is a comparison, not an open bracket, so the '>=' after it is
        // still one operator. Counting it as a bracket wrote 'count > = 1'.
        assert_eq!(format_nail_line("flag_one:b = (age_value < 78) && (score_value >= 10);"), "flag_one:b = (age_value < 78) && (score_value >= 10);");
        // The one place '<' really does open something is a hashmap type.
        assert_eq!(format_nail_line("ages:h<s,i> = hashmap_new();"), "ages:h<s, i> = hashmap_new();");

        // A '(' closes up against a name, because that is a call, and stays
        // apart from a keyword, because 'y (count + 1)' yields a value while
        // 'y(count + 1)' calls a function named y.
        assert_eq!(format_nail_line("print (message_text);"), "print(message_text);");
        assert_eq!(format_nail_line("y (count_value + 1);"), "y (count_value + 1);");
        assert_eq!(format_nail_line("r (count_value + 1);"), "r (count_value + 1);");

        // A keyword before a minus sign means the minus belongs to the number
        // after it: `from-1` starts a fold at minus one, and `from - 1` is a
        // subtraction, which is a different program.
        assert_eq!(format_nail_line("running:a:i = scan acc num in items from-1 { y acc + num; };"), "running:a:i = scan acc num in items from-1 { y acc + num; };");
        assert_eq!(format_nail_line("left_value:i = count_value-1;"), "left_value:i = count_value - 1;");

        // A brace inside a comment is text. Splitting the line there cut the
        // comment in half and left a file that no longer lexes.
        let with_a_brace_in_a_comment = vec![
            "nail latest".to_string(),
            "total_value:i = 2;".to_string(),
            "print(total_value); // prints 2 } and says so".to_string(),
        ];
        assert_eq!(format_nail_code(&with_a_brace_in_a_comment), with_a_brace_in_a_comment);

        // Formatting a whole file twice changes nothing either. A one-line
        // `if` at the top level ends with a brace, and counting that as the
        // end of a block added a blank line on every pass.
        let with_a_one_line_if = vec![
            "nail latest".to_string(),
            "count_value:i = 4;".to_string(),
            "if { count_value > 3 -> { print(`big`); }, else -> { print(`small`); } }".to_string(),
            "print(count_value);".to_string(),
        ];
        let once = format_nail_code(&with_a_one_line_if);
        assert_eq!(format_nail_code(&once), once);

        // Formatting an already formatted line changes nothing. '!=' used to
        // add a space every pass, so a file grew a space each time it was
        // saved and the formatter never reached a fixed point.
        for line in ["print(3 != 3.5);", "count_one:i = 2 + 3;", "flag:b = (left_value >= right_value);", "half:i = total_value / 2;"] {
            assert_eq!(format_nail_line(&format_nail_line(line)), format_nail_line(line), "formatting '{}' twice differs from once", line);
        }
    }
}
