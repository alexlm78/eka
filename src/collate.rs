//! Locale-aware name ordering, an approximation of what GNU ls gets from
//! `strcoll` in a UTF-8 locale. Names compare ignoring case and accents first,
//! and only ties are broken by accents, then case (lowercase first), then the
//! raw bytes. In the `C` and `POSIX` locales names keep comparing bytewise.
//!
//! Two models, because the C libraries differ:
//! - macOS: punctuation sorts before digits, digits before letters.
//! - glibc (Linux): punctuation is ignored at first, so `.h` sorts as `h`; only
//!   after the letters, accents and case tie does the punctuation decide, in
//!   the order of `GLIBC_PUNCTUATION`, with a name that has punctuation before
//!   the same name without it at that position (`-a` < `a`, `a` < `a-`).

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

/// Order of ASCII punctuation in glibc's `en_US.UTF-8`, measured on Ubuntu 24.04
const GLIBC_PUNCTUATION: &str = " !\"#%&'()*+,-.:;<=>?@[\\]^_`{|}~";

/// Accented forms per base letter in glibc's order (acute, grave, breve,
/// circumflex, caron, ring, diaeresis, ...); the base letter itself sorts first
const GLIBC_FOLDS: [(char, &str); 8] = [
    ('a', "áàăâåäãąā"),
    ('c', "ćĉčċç"),
    ('e', "éèĕêěëėęē"),
    ('i', "íìĭîïĩįīı"),
    ('n', "ńňñņ"),
    ('o', "óòŏôöőõøō"),
    ('u', "úùŭûůüűũųū"),
    ('y', "ýŷÿ"),
];

/// Which C library's ordering to follow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Macos,
    Glibc,
}

impl Model {
    /// The model of the platform eka runs on
    pub const fn native() -> Self {
        if cfg!(target_os = "macos") {
            Model::Macos
        } else {
            Model::Glibc
        }
    }
}

/// Sort key: comparing keys with `Ord` gives the collation order
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CollationKey {
    base: Vec<u32>,
    accents: Vec<u8>,
    case: Vec<u8>,
    /// glibc only: the punctuation and letters as a last-level sequence
    tail: Vec<u32>,
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
    key_with(name, Model::native())
}

pub fn key_with(name: &[u8], model: Model) -> CollationKey {
    let text = String::from_utf8_lossy(name);
    let mut base = Vec::with_capacity(text.len());
    let mut accents = Vec::with_capacity(text.len());
    let mut case = Vec::with_capacity(text.len());
    let mut tail = Vec::new();
    for (c, extra) in expand(&text, model) {
        let lower = c.to_lowercase().next().unwrap_or(c);
        if model == Model::Glibc {
            if !c.is_alphanumeric() && c != '$' {
                // Ignored at the first levels; `$` is the exception, it has
                // a primary weight below the digits
                tail.push(glibc_punctuation(c));
                continue;
            }
            tail.push(1000);
        }
        let (letter, accent) = fold(lower, model);
        base.push(primary(letter));
        accents.push(accent.max(extra));
        case.push(u8::from(lower != c));
    }
    CollationKey {
        base,
        accents,
        case,
        tail,
        raw: name.to_vec(),
    }
}

/// The characters to weigh, each with an accent rank that is added to the
/// character's own. glibc spells out ligatures (`æ` sorts as `ae`) and ranks
/// them after every accented form of the same letters.
fn expand(text: &str, model: Model) -> Vec<(char, u8)> {
    const LIGATURE: u8 = 99;
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match (model, c) {
            (Model::Glibc, 'æ') => out.extend([('a', LIGATURE), ('e', 0)]),
            (Model::Glibc, 'Æ') => out.extend([('A', LIGATURE), ('E', 0)]),
            (Model::Glibc, 'œ') => out.extend([('o', LIGATURE), ('e', 0)]),
            (Model::Glibc, 'Œ') => out.extend([('O', LIGATURE), ('E', 0)]),
            (Model::Glibc, 'ß') => out.extend([('s', LIGATURE), ('s', 0)]),
            _ => out.push((c, 0)),
        }
    }
    out
}

/// Last-level weight of a character that is not a letter or a digit
fn glibc_punctuation(c: char) -> u32 {
    GLIBC_PUNCTUATION
        .chars()
        .position(|p| p == c)
        .map_or(100 + c as u32 % 800, |i| 1 + i as u32)
}

/// Base letter of an accented lowercase letter and its accent rank
fn fold(c: char, model: Model) -> (char, u8) {
    if c.is_ascii() {
        return (c, 0);
    }
    let table = if model == Model::Glibc {
        &GLIBC_FOLDS
    } else {
        &FOLDS
    };
    for (base, accented) in table {
        if let Some(i) = accented.chars().position(|a| a == c) {
            return (*base, i as u8 + 1);
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

    fn sorted(model: Model, names: &[&str]) -> Vec<String> {
        let mut v: Vec<&str> = names.to_vec();
        v.sort_by_cached_key(|n| key_with(n.as_bytes(), model));
        v.into_iter().map(String::from).collect()
    }

    #[test]
    fn macos_model_puts_punctuation_before_digits() {
        // Orders observed from GNU ls 9.12 on macOS with LC_ALL=en_US.UTF-8
        assert_eq!(
            sorted(
                Model::Macos,
                &[
                    "B", "a", "c", "é", "e", "ñ", "n", "z", "10", "2", "1", "_x", ".h"
                ]
            ),
            [
                "_x", ".h", "1", "10", "2", "a", "B", "c", "e", "é", "n", "ñ", "z"
            ]
        );
        assert_eq!(
            sorted(Model::Macos, &["ab~", "ab", "a.b", "a-b", "a_b", "b", "a"]),
            ["a", "a_b", "a-b", "a.b", "ab", "ab~", "b"]
        );
    }

    #[test]
    fn glibc_model_ignores_punctuation_first() {
        // Orders observed from GNU ls 9.4 on Ubuntu 24.04 (glibc 2.39)
        assert_eq!(
            sorted(
                Model::Glibc,
                &["x", "_x", "h", ".h", "f", "1", "10", "2", "e", "é"]
            ),
            ["1", "10", "2", "e", "é", "f", ".h", "h", "_x", "x"]
        );
        assert_eq!(
            sorted(
                Model::Glibc,
                &[
                    "a_b", "a-b", "a.b", "a b", "ab", "ab ", "ab~", "aB", "Ab", "A", "a", "a-"
                ]
            ),
            [
                "a", "a-", "A", "a b", "a-b", "a.b", "a_b", "ab", "ab ", "ab~", "aB", "Ab"
            ]
        );
        assert_eq!(
            sorted(Model::Glibc, &["a", "-a", ".a", "_a", "a-", "a.", "a_"]),
            ["-a", ".a", "_a", "a", "a-", "a.", "a_"]
        );
        // Accents in glibc's order: acute, grave, breve, circumflex, ring, diaeresis
        assert_eq!(
            sorted(Model::Glibc, &["ä", "å", "â", "ă", "à", "á", "a"]),
            ["a", "á", "à", "ă", "â", "å", "ä"]
        );
    }

    #[test]
    fn ties_break_by_accent_then_case_then_bytes() {
        for model in [Model::Macos, Model::Glibc] {
            assert_eq!(
                sorted(model, &["É", "B", "é", "b", "E", "e"]),
                ["b", "B", "e", "E", "é", "É"],
                "{model:?}"
            );
        }
    }

    #[test]
    fn glibc_model_weighs_dollar_and_ligatures() {
        // Orders observed from GNU ls 9.4 on Ubuntu 24.04, LANG=en_US.UTF-8
        assert_eq!(
            sorted(Model::Glibc, &["a", "1", "$", "-", "$a", "b"]),
            ["-", "$", "$a", "1", "a", "b"]
        );
        assert_eq!(
            sorted(Model::Glibc, &["æ", "b", "aÉ", "ad", "af"]),
            ["ad", "aÉ", "æ", "af", "b"]
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
