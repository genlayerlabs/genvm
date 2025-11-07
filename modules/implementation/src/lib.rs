pub mod common;
pub mod scripting;

pub use scripting::filters;

pub fn complete_json(partial: &str) -> String {
    let mut result = partial.trim_end().to_string();

    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escape_next = false;
    let mut has_colon = false;

    for ch in partial.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => stack.push('}'),
            '[' if !in_string => stack.push(']'),
            '}' if !in_string => {
                stack.pop();
            }
            ']' if !in_string => {
                stack.pop();
            }
            ':' if !in_string => {
                has_colon = true;
            }
            ',' if !in_string && stack.last() == Some(&'}') => {
                has_colon = false;
            }
            _ => {}
        }
    }

    if in_string {
        if escape_next {
            result.pop();
        }

        let mut s_rev = result.chars().rev();

        let mut hex_count = 0;
        let mut had_u = false;
        for _i in 0..4 {
            if let Some(ch) = s_rev.next() {
                if ch.is_digit(16) {
                    hex_count += 1;
                } else {
                    if ch == 'u' || ch == 'U' {
                        had_u = true;
                    }

                    break;
                }
            } else {
                break;
            }
        }

        if had_u {
            let mut slashes = 0;

            while s_rev.next() == Some('\\') {
                slashes += 1;
            }

            if slashes % 2 == 1 {
                result.truncate(result.len() - 2 - hex_count);
            }
        }

        result.push('"');
    }

    let trimmed_len = result.trim_end().len();
    result.truncate(trimmed_len);

    if result.ends_with('{') {
        // nothing
    } else if result.ends_with(':') {
        result.push_str("null");
    } else if result.ends_with('-') || result.ends_with('.') || result.ends_with('+') {
        result.push('0');
    } else if (result.ends_with('e') || result.ends_with('E'))
        && result.chars().rev().nth(1).unwrap_or('a').is_digit(10)
    {
        result.push_str("0");
    } else if result.ends_with(',') {
        result.pop();
    } else if stack.last() == Some(&'}') && !has_colon && !result.ends_with('{') {
        result.push_str(":null");
    } else {
        const CONSTS: &[&str] = &["true", "false", "null"];

        for &c in CONSTS {
            for i in 1..c.len() {
                if result.ends_with(&c[..c.len() - i]) {
                    result.push_str(&c[c.len() - i..]);
                    break;
                }
            }
            if result.ends_with(c) {
                break;
            }
        }
    }

    while let Some(closing) = stack.pop() {
        result.push(closing);
    }

    if result.is_empty() {
        result.push_str("{}");
    }

    result
}
