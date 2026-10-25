//! Locale-aware name ordering, an approximation of what GNU ls gets from
//! `strcoll` in a UTF-8 locale. Names compare ignoring case and accents first
//! (punctuation, then digits, then letters), and only ties are broken by
//! accents, then case (lowercase first), then the raw bytes. In the `C` and
//! `POSIX` locales names keep comparing bytewise.

use std::sync::OnceLock;

/// CLDR root order of ASCII punctuation, which sorts before digits
const PUNCTUATION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

/// Lowercase letters that fold to a base letter, in a fixed order that doubles
/// as the accent rank (unaccented sorts first)
const FOLDS: [(char, &str); 8] = [
    ('a', "àáâãäåāăą"),
    ('c', "çćĉċč"),
    ('e', "èéêëēĕėęě"),
    ('i', "ìíîïĩīĭįı"),
    ('n', "ñńņň"),
    ('o', "òóôõöøōŏő"),
    ('u', "ùúûüũūŭůűų"),
    ('y', "ýÿŷ"),
];

/// Sort key: comparing keys with `Ord` gives the collation order
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CollationKey {
    base: Vec<u32>,
    accents: Vec<u8>,
    case: Vec<u8>,
    raw: Vec<u8>,
}

/// Whether the environment asks for a collating locale: `LC_ALL`, then
/// `LC_COLLATE`, then `LANG`, the first one that is set and not empty
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let value = ["LC_ALL", "LC_COLLATE", "LANG"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|v| !v.is_empty());
        value.is_some_and(|v| is_collating(&v))
    })
}

fn is_collating(locale: &str) -> bool {
    !(locale == "C" || locale == "POSIX" || locale.starts_with("C.") || locale.starts_with("C@"))
}

pub fn key(name: &[u8]) -> CollationKey {
    let text = String::from_utf8_lossy(name);
    let mut base = Vec::with_capacity(text.len());
    let mut accents = Vec::with_capacity(text.len());
    let mut case = Vec::with_capacity(text.len());
    for c in text.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        let (letter, accent) = fold(lower);
        base.push(primary(letter));
        accents.push(accent);
        case.push(u8::from(lower != c));
    }
    CollationKey {
        base,
        accents,
        case,
        raw: name.to_vec(),
    }
}

/// Base letter of an accented lowercase letter and its accent rank
fn fold(c: char) -> (char, u8) {
    if c.is_ascii() {
        return (c, 0);
    }
    for (base, accented) in FOLDS {
        if let Some(i) = accented.chars().position(|a| a == c) {
            return (base, i as u8 + 1);
        }
    }
    (c, 0)
}

fn primary(c: char) -> u32 {
    if c.is_whitespace() {
        1
    } else if let Some(i) = PUNCTUATION.chars().position(|p| p == c) {
        2 + i as u32
    } else if c.is_ascii_digit() {
        100 + (c as u32 - '0' as u32)
    } else if c.is_ascii_lowercase() {
        200 + (c as u32 - 'a' as u32)
    } else if c.is_alphanumeric() {
        1000 + c as u32
    } else if c.is_control() {
        0
    } else {
        60 // other symbols sort between punctuation and digits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut v: Vec<&str> = names.to_vec();
        v.sort_by_cached_key(|n| key(n.as_bytes()));
        v.into_iter().map(String::from).collect()
    }

    #[test]
    fn matches_gnu_ls_in_a_utf8_locale() {
        // Orders observed from GNU ls 9.12 with LC_ALL=en_US.UTF-8
        assert_eq!(
            sorted(&[
                "B", "a", "c", "é", "e", "ñ", "n", "z", "10", "2", "1", "_x", ".h"
            ]),
            [
                "_x", ".h", "1", "10", "2", "a", "B", "c", "e", "é", "n", "ñ", "z"
            ]
        );
        assert_eq!(
            sorted(&["ab~", "ab", "a.b", "a-b", "a_b", "b", "a"]),
            ["a", "a_b", "a-b", "a.b", "ab", "ab~", "b"]
        );
    }

    #[test]
    fn ties_break_by_accent_then_case_then_bytes() {
        assert_eq!(
            sorted(&["É", "B", "é", "b", "E", "e"]),
            ["b", "B", "e", "E", "é", "É"]
        );
    }

    #[test]
    fn c_and_posix_do_not_collate() {
        for l in ["C", "POSIX", "C.UTF-8", "C.utf8"] {
            assert!(!is_collating(l), "{l}");
        }
        for l in ["en_US.UTF-8", "es_ES.UTF-8", "de_DE"] {
            assert!(is_collating(l), "{l}");
        }
    }
}
