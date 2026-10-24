//! Small byte-oriented shell-pattern matcher.

/// Returns whether `name` matches the complete shell pattern.
pub fn matches(pattern: &[u8], name: &[u8]) -> bool {
    let mut pattern_index = 0;
    let mut name_index = 0;
    let mut star = None;
    let mut star_name_index = 0;

    while name_index < name.len() {
        if pattern.get(pattern_index) == Some(&b'*') {
            star = Some(pattern_index);
            pattern_index += 1;
            star_name_index = name_index;
        } else if let Some(next) = token_matches(pattern, pattern_index, name[name_index]) {
            pattern_index = next;
            name_index += 1;
        } else if let Some(star_index) = star {
            star_name_index += 1;
            name_index = star_name_index;
            pattern_index = star_index + 1;
        } else {
            return false;
        }
    }

    while pattern.get(pattern_index) == Some(&b'*') {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

/// Returns the next pattern index when one pattern token matches `byte`.
fn token_matches(pattern: &[u8], index: usize, byte: u8) -> Option<usize> {
    match pattern.get(index)? {
        b'?' => Some(index + 1),
        b'\\' => match pattern.get(index + 1) {
            Some(escaped) if *escaped == byte => Some(index + 2),
            None if byte == b'\\' => Some(index + 1),
            _ => None,
        },
        b'[' => match class_end(pattern, index) {
            Some(_) => class_matches(pattern, index, byte),
            None if byte == b'[' => Some(index + 1),
            None => None,
        },
        literal if *literal == byte => Some(index + 1),
        _ => None,
    }
}

fn class_matches(pattern: &[u8], index: usize, byte: u8) -> Option<usize> {
    let end = class_end(pattern, index)?;
    if end == index + 1 {
        return None;
    }
    let mut cursor = index + 1;
    let negated = matches!(pattern.get(cursor), Some(b'!' | b'^'));
    if negated {
        cursor += 1;
    }
    if cursor == end {
        return None;
    }
    let mut found = false;
    while cursor < end {
        let (first, next) = class_byte(pattern, cursor, end);
        cursor = next;
        if cursor < end && pattern[cursor] == b'-' && cursor + 1 < end {
            let (last, next) = class_byte(pattern, cursor + 1, end);
            found |= first <= byte && byte <= last;
            cursor = next;
        } else {
            found |= first == byte;
        }
    }
    if found != negated {
        Some(end + 1)
    } else {
        None
    }
}

fn class_end(pattern: &[u8], index: usize) -> Option<usize> {
    let mut cursor = index + 1;
    if matches!(pattern.get(cursor), Some(b'!' | b'^')) {
        cursor += 1;
    }
    // A leading `]` is a literal member, as in `[]a]`
    if pattern.get(cursor) == Some(&b']') {
        cursor += 1;
    }
    while cursor < pattern.len() {
        if pattern[cursor] == b'\\' && cursor + 1 < pattern.len() {
            cursor += 2;
        } else if pattern[cursor] == b']' {
            return Some(cursor);
        } else {
            cursor += 1;
        }
    }
    None
}

fn class_byte(pattern: &[u8], index: usize, end: usize) -> (u8, usize) {
    if pattern[index] == b'\\' && index + 1 < end {
        (pattern[index + 1], index + 2)
    } else {
        (pattern[index], index + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn test_empty_and_wildcards() {
        assert!(matches(b"", b""));
        assert!(!matches(b"", b"x"));
        assert!(matches(b"*", b".hidden"));
        assert!(matches(b"a?c", b"abc"));
    }

    #[test]
    fn test_escapes_and_classes() {
        assert!(matches(br"a\*", b"a*"));
        assert!(matches(br"a\?", b"a?"));
        assert!(matches(b"[abc]", b"b"));
        assert!(matches(b"[a-z]", b"m"));
        assert!(matches(b"[!abc]", b"z"));
        assert!(matches(b"[^abc]", b"z"));
        assert!(!matches(b"[!abc]", b"a"));
        assert!(matches(br"[\]]", b"]"));
        assert!(matches(b"[", b"["));
    }

    #[test]
    fn test_leading_close_bracket_is_literal_member() {
        assert!(matches(b"[]a]", b"]"));
        assert!(matches(b"[]a]", b"a"));
        assert!(!matches(b"[]a]", b"b"));
        assert!(matches(b"[!]a]", b"b"));
        assert!(!matches(b"[!]a]", b"]"));
        // no closing bracket after the literal one: plain `[`
        assert!(matches(b"[]", b"[]"));
    }

    #[test]
    fn test_non_utf8_and_pathological_input() {
        assert!(matches(&[b'*', 0xff], &[b'a', 0xff]));
        assert!(!matches(&[0xff], &[0xfe]));
        assert!(!matches(b"*a*a*a*a*a*a*b", &vec![b'a'; 10_000]));
    }
}
