//! Quoting and escaping of file names for output.
//!
//! File names are attacker-controlled bytes. Printed raw to a terminal, a
//! name can move the cursor, clear the screen, set the window title or
//! reorder text with bidirectional overrides. This module turns a name into
//! bytes that are safe to print, following the GNU `--quoting-style` rules.

use clap::ValueEnum;

/// How names are quoted (`--quoting-style`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum QuotingStyle {
    /// Print names as they are
    #[default]
    Literal,
    /// Quote with single quotes only when the shell would need it
    Shell,
    /// Always quote with single quotes
    ShellAlways,
    /// Like `shell`, with ANSI-C escapes for unprintable characters
    ShellEscape,
    /// Like `shell-always`, with ANSI-C escapes for unprintable characters
    ShellEscapeAlways,
    /// C string literal in double quotes
    C,
    /// Backslash escapes without quotes
    Escape,
}

/// Characters that make a name need quoting in the shell styles
const SHELL_SPECIAL: &[char] = &[
    ' ', '\t', '\n', '!', '"', '$', '&', '\'', '(', ')', '*', ';', '<', '=', '>', '?', '[', '\\',
    '^', '`', '|', '{', '}',
];

/// One decoded unit of a name: a character or a byte that is not valid UTF-8
#[derive(Debug, Clone, Copy)]
enum Piece {
    Ch(char),
    Bad(u8),
}

fn pieces(name: &[u8]) -> impl Iterator<Item = Piece> + '_ {
    name.utf8_chunks().flat_map(|chunk| {
        chunk
            .valid()
            .chars()
            .map(Piece::Ch)
            .chain(chunk.invalid().iter().map(|&b| Piece::Bad(b)))
    })
}

/// Whether a character is safe to show as is. Besides control characters
/// this excludes the bidirectional overrides and isolates (they can make
/// `evil\u{202e}txt.exe` look like `evilexe.txt`) and the line separators.
fn printable(c: char) -> bool {
    !c.is_control()
        && !matches!(
            c as u32,
            0x061C | 0x200E | 0x200F | 0x2028 | 0x2029 | 0x202A..=0x202E | 0x2066..=0x2069
        )
}

impl Piece {
    fn is_printable(self) -> bool {
        matches!(self, Piece::Ch(c) if printable(c))
    }
}

/// Appends the backslash escape for an unprintable piece (`\n`, `\033`, ...)
fn push_escape(out: &mut Vec<u8>, piece: Piece) {
    let short = match piece {
        Piece::Ch('\x07') => Some(b'a'),
        Piece::Ch('\x08') => Some(b'b'),
        Piece::Ch('\t') => Some(b't'),
        Piece::Ch('\n') => Some(b'n'),
        Piece::Ch('\x0b') => Some(b'v'),
        Piece::Ch('\x0c') => Some(b'f'),
        Piece::Ch('\r') => Some(b'r'),
        _ => None,
    };
    if let Some(letter) = short {
        out.extend_from_slice(&[b'\\', letter]);
        return;
    }
    // Octal per byte, so multibyte characters stay unambiguous
    let mut buf = [0u8; 4];
    let bytes: &[u8] = match piece {
        Piece::Ch(c) => c.encode_utf8(&mut buf).as_bytes(),
        Piece::Bad(b) => {
            buf[0] = b;
            &buf[..1]
        }
    };
    for b in bytes {
        out.extend_from_slice(format!("\\{:03o}", b).as_bytes());
    }
}

fn push_piece_raw(out: &mut Vec<u8>, piece: Piece) {
    match piece {
        Piece::Ch(c) => {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
        Piece::Bad(b) => out.push(b),
    }
}

/// Turns names into printable bytes
#[derive(Debug, Clone, Copy)]
pub struct Quoter {
    style: QuotingStyle,
    /// Replace unprintable characters with `?` in the styles that print them
    hide_control: bool,
}

impl Quoter {
    pub fn new(style: QuotingStyle, hide_control: bool) -> Self {
        Self {
            style,
            hide_control,
        }
    }

    /// Quotes `name` for output
    pub fn quote(&self, name: &[u8]) -> Vec<u8> {
        match self.style {
            QuotingStyle::Literal => self.literal(name),
            QuotingStyle::Escape => escape(name, true),
            QuotingStyle::C => c_style(name),
            QuotingStyle::Shell => self.shell(name, false, false),
            QuotingStyle::ShellAlways => self.shell(name, true, false),
            QuotingStyle::ShellEscape => self.shell(name, false, true),
            QuotingStyle::ShellEscapeAlways => self.shell(name, true, true),
        }
    }

    /// Quotes a directory heading. Like GNU ls, the escape style does not
    /// escape spaces there (it does in file names).
    pub fn quote_heading(&self, name: &[u8]) -> Vec<u8> {
        match self.style {
            QuotingStyle::Escape => escape(name, false),
            _ => self.quote(name),
        }
    }

    /// Whether `quote(name)` starts with a quote that other names in the
    /// listing may not have. Used to align unquoted names, as GNU ls does.
    pub fn is_quoted(&self, name: &[u8]) -> bool {
        matches!(self.style, QuotingStyle::Shell | QuotingStyle::ShellEscape)
            && needs_shell_quotes(name)
    }

    /// True for the styles where quoted and unquoted names mix in one listing
    pub fn aligns(&self) -> bool {
        matches!(self.style, QuotingStyle::Shell | QuotingStyle::ShellEscape)
    }

    /// Whether a piece is printed untouched in the styles that show names
    /// as they are. A newline is always replaced, as in GNU ls: left alone it
    /// would split one entry across two lines.
    fn shows_raw(&self, piece: Piece) -> bool {
        piece.is_printable() || (!self.hide_control && !matches!(piece, Piece::Ch('\n')))
    }

    fn literal(&self, name: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(name.len());
        for p in pieces(name) {
            if self.shows_raw(p) {
                push_piece_raw(&mut out, p);
            } else {
                out.push(b'?');
            }
        }
        out
    }

    fn shell(&self, name: &[u8], always: bool, escape: bool) -> Vec<u8> {
        let has_unprintable = pieces(name).any(|p| !p.is_printable());
        if !always && !needs_shell_quotes(name) {
            return name.to_vec();
        }
        let mut out = Vec::with_capacity(name.len() + 2);
        if escape && has_unprintable {
            // 'printable run'$'\n'... : single quotes and ANSI-C quotes alternate
            let (mut in_single, mut in_ansi) = (false, false);
            for p in pieces(name) {
                if p.is_printable() {
                    if in_ansi {
                        out.push(b'\'');
                        in_ansi = false;
                    }
                    if !in_single {
                        out.push(b'\'');
                        in_single = true;
                    }
                    match p {
                        Piece::Ch('\'') => out.extend_from_slice(b"'\\''"),
                        _ => push_piece_raw(&mut out, p),
                    }
                } else {
                    if in_single {
                        out.push(b'\'');
                        in_single = false;
                    }
                    if !in_ansi {
                        out.extend_from_slice(b"$'");
                        in_ansi = true;
                    }
                    push_escape(&mut out, p);
                }
            }
            if in_single || in_ansi {
                out.push(b'\'');
            }
            return out;
        }
        out.push(b'\'');
        for p in pieces(name) {
            match p {
                Piece::Ch('\'') => out.extend_from_slice(b"'\\''"),
                p if self.shows_raw(p) => push_piece_raw(&mut out, p),
                _ => out.push(b'?'),
            }
        }
        out.push(b'\'');
        out
    }
}

/// Whether the shell styles must quote this name
fn needs_shell_quotes(name: &[u8]) -> bool {
    if name.is_empty() || matches!(name[0], b'#' | b'~') {
        return true;
    }
    pieces(name).any(|p| match p {
        Piece::Ch(c) => !printable(c) || SHELL_SPECIAL.contains(&c),
        Piece::Bad(_) => true,
    })
}

fn escape(name: &[u8], escape_space: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(name.len());
    for p in pieces(name) {
        match p {
            Piece::Ch('\\') => out.extend_from_slice(b"\\\\"),
            Piece::Ch(' ') if escape_space => out.extend_from_slice(b"\\ "),
            p if p.is_printable() => push_piece_raw(&mut out, p),
            p => push_escape(&mut out, p),
        }
    }
    out
}

fn c_style(name: &[u8]) -> Vec<u8> {
    let mut out = vec![b'"'];
    for p in pieces(name) {
        match p {
            Piece::Ch('"') => out.extend_from_slice(b"\\\""),
            Piece::Ch('\\') => out.extend_from_slice(b"\\\\"),
            p if p.is_printable() => push_piece_raw(&mut out, p),
            p => push_escape(&mut out, p),
        }
    }
    out.push(b'"');
    out
}

/// Quotes a path for an error message. Always safe to print: control
/// characters and invalid bytes are escaped, and the result is valid UTF-8.
pub fn quote_for_message(name: &[u8]) -> String {
    let quoted = Quoter::new(QuotingStyle::ShellEscapeAlways, true).quote(name);
    // Only valid characters and ASCII escapes are produced
    String::from_utf8_lossy(&quoted).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(style: QuotingStyle, hide: bool, name: &[u8]) -> String {
        String::from_utf8_lossy(&Quoter::new(style, hide).quote(name)).into_owned()
    }

    #[test]
    fn test_literal_keeps_names_and_optionally_hides_control() {
        assert_eq!(q(QuotingStyle::Literal, false, b"a b"), "a b");
        assert_eq!(q(QuotingStyle::Literal, false, b"a\x1bb"), "a\x1bb");
        // a newline would split the entry across lines, so it is never raw
        assert_eq!(q(QuotingStyle::Literal, false, b"a\nb"), "a?b");
        assert_eq!(q(QuotingStyle::Shell, false, b"a\nb"), "'a?b'");
        assert_eq!(q(QuotingStyle::ShellAlways, false, b"a\nb"), "'a?b'");
        assert_eq!(q(QuotingStyle::Literal, true, b"a\x1b[2Jb"), "a?[2Jb");
        assert_eq!(q(QuotingStyle::Literal, true, b"a\xffb"), "a?b");
        assert_eq!(
            q(QuotingStyle::Literal, true, "caf\u{e9}".as_bytes()),
            "caf\u{e9}"
        );
    }

    #[test]
    fn test_bidi_and_c1_controls_are_unprintable() {
        for c in [
            '\u{202e}', '\u{2066}', '\u{200f}', '\u{2028}', '\u{85}', '\u{7f}',
        ] {
            let name = format!("a{c}b");
            assert_eq!(
                q(QuotingStyle::Literal, true, name.as_bytes()),
                "a?b",
                "{c:?}"
            );
        }
        // joiners used by emoji sequences stay printable
        let family = "\u{1f468}\u{200d}\u{1f469}";
        assert_eq!(q(QuotingStyle::Literal, true, family.as_bytes()), family);
    }

    #[test]
    fn test_escape_style() {
        assert_eq!(q(QuotingStyle::Escape, false, b"a b"), "a\\ b");
        assert_eq!(q(QuotingStyle::Escape, false, b"a\\b"), "a\\\\b");
        assert_eq!(q(QuotingStyle::Escape, false, b"a\nb\tc"), "a\\nb\\tc");
        assert_eq!(q(QuotingStyle::Escape, false, b"\x1b[0m"), "\\033[0m");
        assert_eq!(q(QuotingStyle::Escape, false, b"x\xff"), "x\\377");
        assert_eq!(
            q(QuotingStyle::Escape, false, "\u{202e}".as_bytes()),
            "\\342\\200\\256"
        );
        assert_eq!(
            q(QuotingStyle::Escape, false, "caf\u{e9}".as_bytes()),
            "caf\u{e9}"
        );
    }

    #[test]
    fn test_heading_does_not_escape_spaces_in_escape_style() {
        let quoter = Quoter::new(QuotingStyle::Escape, false);
        assert_eq!(quoter.quote(b"./d e"), b"./d\\ e");
        assert_eq!(quoter.quote_heading(b"./d e"), b"./d e");
        // control characters are still escaped
        assert_eq!(quoter.quote_heading(b"d\x1b"), b"d\\033");
        // other styles are unchanged
        let shell = Quoter::new(QuotingStyle::ShellEscape, false);
        assert_eq!(shell.quote_heading(b"./d e"), shell.quote(b"./d e"));
    }

    #[test]
    fn test_c_style() {
        assert_eq!(q(QuotingStyle::C, false, b"plain"), "\"plain\"");
        assert_eq!(q(QuotingStyle::C, false, b"a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(q(QuotingStyle::C, false, b"a b\n"), "\"a b\\n\"");
        assert_eq!(q(QuotingStyle::C, false, b""), "\"\"");
    }

    #[test]
    fn test_shell_quotes_only_when_needed() {
        assert_eq!(q(QuotingStyle::Shell, false, b"plain.txt"), "plain.txt");
        assert_eq!(q(QuotingStyle::Shell, false, b"a b"), "'a b'");
        assert_eq!(q(QuotingStyle::Shell, false, b""), "''");
        assert_eq!(q(QuotingStyle::Shell, false, b"it's"), "'it'\\''s'");
        assert_eq!(q(QuotingStyle::Shell, false, b"~home"), "'~home'");
        assert_eq!(q(QuotingStyle::Shell, false, b"a~"), "a~");
        assert_eq!(q(QuotingStyle::Shell, false, b"#c"), "'#c'");
        for special in [
            "a$b", "a*b", "a?b", "a;b", "a|b", "a&b", "a`b", "a(b", "a<b", "a=b",
        ] {
            assert!(q(QuotingStyle::Shell, false, special.as_bytes()).starts_with('\''));
        }
        assert_eq!(q(QuotingStyle::ShellAlways, false, b"plain"), "'plain'");
    }

    #[test]
    fn test_shell_escape_uses_ansi_c_quotes_for_unprintable() {
        assert_eq!(q(QuotingStyle::ShellEscape, false, b"a\nb"), "'a'$'\\n''b'");
        assert_eq!(q(QuotingStyle::ShellEscape, false, b"\n"), "$'\\n'");
        assert_eq!(q(QuotingStyle::ShellEscape, false, b"a\n"), "'a'$'\\n'");
        assert_eq!(
            q(QuotingStyle::ShellEscape, false, b"\x1b[2J"),
            "$'\\033''[2J'"
        );
        assert_eq!(
            q(QuotingStyle::ShellEscape, false, b"it's\n"),
            "'it'\\''s'$'\\n'"
        );
        assert_eq!(q(QuotingStyle::ShellEscape, false, b"x\xff"), "'x'$'\\377'");
        assert_eq!(q(QuotingStyle::ShellEscape, false, b"plain"), "plain");
        assert_eq!(q(QuotingStyle::ShellEscape, false, b"a b"), "'a b'");
        assert_eq!(
            q(QuotingStyle::ShellEscapeAlways, false, b"plain"),
            "'plain'"
        );
    }

    #[test]
    fn test_plain_shell_style_never_emits_raw_control_when_hiding() {
        assert_eq!(q(QuotingStyle::Shell, true, b"a\x1bb"), "'a?b'");
    }

    #[test]
    fn test_alignment_helpers() {
        let q = Quoter::new(QuotingStyle::ShellEscape, true);
        assert!(q.aligns() && q.is_quoted(b"a b") && !q.is_quoted(b"ab"));
        let always = Quoter::new(QuotingStyle::ShellEscapeAlways, true);
        assert!(!always.aligns() && !always.is_quoted(b"a b"));
        assert!(!Quoter::new(QuotingStyle::Literal, true).aligns());
    }

    #[test]
    fn test_message_quoting_is_always_safe() {
        let m = quote_for_message(b"/tmp/x\x1b]0;pwn\x07\xff");
        assert!(!m.bytes().any(|b| b < 0x20 || b == 0x7f), "{m:?}");
        assert!(m.contains("\\033") && m.contains("\\377"));
        assert_eq!(quote_for_message(b"ok"), "'ok'");
    }

    #[test]
    fn test_no_style_emits_raw_escape_when_it_should_not() {
        let evil: &[u8] = b"a\x1b[31m\x9b\xe2\x80\xaeb";
        for style in [
            QuotingStyle::Escape,
            QuotingStyle::C,
            QuotingStyle::ShellEscape,
            QuotingStyle::ShellEscapeAlways,
        ] {
            let out = Quoter::new(style, false).quote(evil);
            assert!(!out.contains(&0x1b), "{style:?}");
            assert!(std::str::from_utf8(&out).is_ok(), "{style:?}");
            assert!(
                !String::from_utf8_lossy(&out).contains('\u{202e}'),
                "{style:?}"
            );
        }
        for style in [
            QuotingStyle::Literal,
            QuotingStyle::Shell,
            QuotingStyle::ShellAlways,
        ] {
            let out = Quoter::new(style, true).quote(evil);
            assert!(!out.contains(&0x1b), "{style:?}");
            assert!(
                !String::from_utf8_lossy(&out).contains('\u{202e}'),
                "{style:?}"
            );
        }
    }
}
