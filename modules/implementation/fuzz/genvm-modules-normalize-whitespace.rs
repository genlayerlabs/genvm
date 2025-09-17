fn check_normalized_invariants(s: &str) {
    // Invariant 1: Only space and newline are allowed as whitespace characters
    for ch in s.chars() {
        if ch.is_whitespace() {
            assert!(
                ch == ' ' || ch == '\n',
                "Found whitespace character {:?} that is not space or newline",
                ch
            );
        }
    }

    // Invariant 2: No line-trailing spaces
    // Check for spaces before newlines
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == '\n' && i > 0 {
            assert!(
                chars[i - 1] != ' ',
                "Found space before newline at position {}",
                i - 1
            );
        }
    }

    // Check for trailing spaces at the end of the string
    assert!(!s.ends_with(' '), "String ends with space character");

    // Invariant 3: No multiple consecutive spaces
    assert!(!s.contains("  "), "Found multiple consecutive spaces");

    // Invariant 4: No more than 2 consecutive newlines
    let mut consecutive_newlines = 0;
    let mut max_consecutive_newlines = 0;

    for ch in s.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            max_consecutive_newlines = max_consecutive_newlines.max(consecutive_newlines);
        } else {
            consecutive_newlines = 0;
        }
    }

    assert!(
        max_consecutive_newlines <= 2,
        "Found {} consecutive newlines (max allowed is 2)",
        max_consecutive_newlines
    );

    // Additional check: no leading spaces after newlines (except at string start)
    for i in 0..chars.len() {
        if i > 0 && chars[i - 1] == '\n' && i < chars.len() {
            // After a newline, we shouldn't immediately have spaces unless followed by non-space
            let mut j = i;
            let mut space_count = 0;
            while j < chars.len() && chars[j] == ' ' {
                space_count += 1;
                j += 1;
            }

            // If we found spaces after newline and they're at the end or followed by newline,
            // these are line-leading/trailing spaces that should have been removed
            if space_count > 0 && (j >= chars.len() || chars[j] == '\n') {
                panic!("Found {} space(s) at the beginning of a line that lead to nothing or another newline", space_count);
            }
        }
    }

    // Check for spaces at the beginning of the string that lead to nothing
    if !s.is_empty() {
        let mut i = 0;
        while i < chars.len() && chars[i] == ' ' {
            i += 1;
        }
        assert!(
            i == 0 || (i < chars.len() && chars[i] != '\n'),
            "String starts with spaces that lead to nothing or a newline"
        );
    }
}

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let Ok(s) = std::str::from_utf8(data) else {
            return;
        };

        let res = genvm_modules::filters::apply_filters(
            &s,
            &[genvm_modules::filters::TextFilter::NormalizeWS],
        );

        if res.len() == 0 {
            return;
        }
        assert!(!res.starts_with(' '));
        assert!(!res.starts_with('\n'));
        assert!(!res.ends_with(' '));

        check_normalized_invariants(&res);
    });
}
