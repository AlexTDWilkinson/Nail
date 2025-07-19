pub fn format_nail_code(lines: &[String]) -> Vec<String> {
    let mut formatted_lines = Vec::new();
    let mut indent_level: usize = 0;
    let mut last_line_had_closing_brace = false;
    
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Skip empty lines
        if trimmed.is_empty() {
            formatted_lines.push(String::new());
            last_line_had_closing_brace = false;
            continue;
        }
        
        // Check if this line starts a new block (function, struct, enum, etc.)
        let starts_new_top_level_block = trimmed.starts_with("f ") || 
                                         trimmed.starts_with("struct ") || 
                                         trimmed.starts_with("enum ") ||
                                         trimmed.starts_with("parallel ");
        
        // Only consider 'if' as starting a new block if it's at the top level
        let starts_new_block = starts_new_top_level_block || 
                              (trimmed.starts_with("if ") && indent_level == 0);
        
        // Add blank line before new blocks (except at the beginning or after comments)
        if starts_new_block && i > 0 && !formatted_lines.is_empty() {
            let last_non_empty_idx = formatted_lines.iter().rposition(|l| !l.trim().is_empty());
            if let Some(idx) = last_non_empty_idx {
                let last_line = formatted_lines[idx].trim();
                // Add blank line if:
                // - Previous line ends with } or ; (end of block/statement)
                // - Previous line is a single-line function
                // - Not after a comment section
                if (last_line.ends_with('}') || last_line.ends_with(';') || 
                    (last_line.starts_with("f ") && last_line.contains('{'))) && 
                   !last_line.starts_with("//") {
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
        
        // Track if this line ends with a closing brace at indent level 0
        last_line_had_closing_brace = (trimmed.ends_with('}') || formatted_content.ends_with('}')) 
                                     && indent_level == 0;
        
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
        
        // Format operators
        match ch {
            '=' => {
                // Trim trailing space before operator
                while formatted.ends_with(' ') {
                    formatted.pop();
                }
                
                if chars.peek() == Some(&'=') {
                    // ==
                    formatted.push_str(" == ");
                    chars.next();
                } else if chars.peek() == Some(&'>') {
                    // =>
                    formatted.push_str(" => ");
                    chars.next();
                } else {
                    // =
                    formatted.push_str(" = ");
                }
            }
            '!' => {
                if chars.peek() == Some(&'=') {
                    // !=
                    formatted.push_str(" != ");
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
                    formatted.push_str(" <= ");
                    chars.next();
                } else {
                    // <
                    formatted.push_str(" < ");
                }
            }
            '>' => {
                if chars.peek() == Some(&'=') {
                    // >=
                    formatted.push_str(" >= ");
                    chars.next();
                } else {
                    // >
                    formatted.push_str(" > ");
                }
            }
            '+' | '-' | '*' | '%' => {
                // Don't add spaces around - if it's a negative number
                if ch == '-' && formatted.chars().last().map_or(true, |c| !c.is_alphanumeric() && c != ')') {
                    formatted.push(ch);
                } else {
                    // Trim trailing space before adding operator with spaces
                    while formatted.ends_with(' ') {
                        formatted.pop();
                    }
                    formatted.push(' ');
                    formatted.push(ch);
                    formatted.push(' ');
                }
            }
            '/' => {
                // Check if this is part of a comment
                if chars.peek() == Some(&'/') {
                    // This is handled by the comment check above, but just in case
                    formatted.push(ch);
                } else if chars.peek() == Some(&'p') {
                    // This is /p for parallel end, keep it as one token without spaces
                    formatted.push(ch);
                    formatted.push(chars.next().unwrap()); // consume the 'p'
                } else {
                    // Regular division operator
                    while formatted.ends_with(' ') {
                        formatted.pop();
                    }
                    formatted.push(' ');
                    formatted.push(ch);
                    formatted.push(' ');
                }
            }
            ',' => {
                formatted.push(',');
                formatted.push(' ');
            }
            ';' => {
                // Just push semicolon, no space after (unless followed by comment)
                formatted.push(ch);
            }
            '(' => {
                // Remove space before (
                if formatted.ends_with(' ') {
                    formatted.pop();
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
    
    // Clean up multiple spaces
    while formatted.contains("  ") {
        formatted = formatted.replace("  ", " ");
    }
    
    formatted.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operators() {
        assert_eq!(format_nail_line("x=5"), "x = 5");
        assert_eq!(format_nail_line("a+b"), "a + b");
        assert_eq!(format_nail_line("c-d"), "c - d");
        assert_eq!(format_nail_line("e*f"), "e * f");
        assert_eq!(format_nail_line("g/h"), "g / h");
    }

    #[test]
    fn test_comparison_operators() {
        assert_eq!(format_nail_line("a==b"), "a == b");
        assert_eq!(format_nail_line("c!=d"), "c != d");
        assert_eq!(format_nail_line("e<=f"), "e <= f");
        assert_eq!(format_nail_line("g>=h"), "g >= h");
        assert_eq!(format_nail_line("i<j"), "i < j");
        assert_eq!(format_nail_line("k>l"), "k > l");
    }

    #[test]
    fn test_arrow_operator() {
        assert_eq!(format_nail_line("if x => y"), "if x => y");
        assert_eq!(format_nail_line("case=>result"), "case => result");
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
        assert_eq!(format_nail_line("y = a - -b"), "y = a - -b");
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
    fn test_complex_expression() {
        assert_eq!(
            format_nail_line("sum_squares:i = reduce_int(map_int(nums, square_func),0,add_func)"),
            "sum_squares:i = reduce_int(map_int(nums, square_func), 0, add_func)"
        );
    }

    #[test]
    fn test_empty_and_whitespace_lines() {
        assert_eq!(format_nail_line(""), "");
        assert_eq!(format_nail_line("   "), "");
        assert_eq!(format_nail_line("\t"), "");
    }
    
    #[test]
    fn test_code_indentation() {
        let input = vec![
            "f greet(name:s):s {".to_string(),
            "parts:a:s = [`Hello, `, name, `!`];".to_string(),
            "r string_concat(parts);".to_string(),
            "}".to_string(),
        ];
        
        let expected = vec![
            "f greet(name:s):s {".to_string(),
            "    parts:a:s = [`Hello, `, name, `!`];".to_string(),
            "    r string_concat(parts);".to_string(),
            "}".to_string(),
        ];
        
        assert_eq!(format_nail_code(&input), expected);
    }
    
    #[test]
    fn test_nested_indentation() {
        let input = vec![
            "if {".to_string(),
            "x > 0 => {".to_string(),
            "print(`positive`);".to_string(),
            "},".to_string(),
            "else => {".to_string(),
            "print(`negative`);".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        
        let expected = vec![
            "if {".to_string(),
            "    x > 0 => {".to_string(),
            "        print(`positive`);".to_string(),
            "    },".to_string(),
            "    else => {".to_string(),
            "        print(`negative`);".to_string(),
            "    }".to_string(),
            "}".to_string(),
        ];
        
        assert_eq!(format_nail_code(&input), expected);
    }
    
    #[test]
    fn test_function_spacing() {
        let input = vec![
            "f double_func(n:i):i { r n * 2; }".to_string(),
            "f is_even_func(n:i):b {".to_string(),
            "r n % 2 == 0;".to_string(),
            "}".to_string(),
            "f add_func(acc:i, n:i):i { r acc + n; }".to_string(),
        ];
        
        let expected = vec![
            "f double_func(n:i):i { r n * 2; }".to_string(),
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
    fn test_parallel_syntax() {
        assert_eq!(format_nail_line("p"), "p");
        assert_eq!(format_nail_line("/p"), "/p");
        assert_eq!(format_nail_line("task1:s = `hello`; /p"), "task1:s = `hello`; /p");
        assert_eq!(format_nail_line("p task1:i = 42; task2:s = `test`; /p"), "p task1:i = 42; task2:s = `test`; /p");
    }
}