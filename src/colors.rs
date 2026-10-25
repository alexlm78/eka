use crate::cli::ColorWhen;
use crate::types::{FileEntry, FileType};
use std::io::{self, IsTerminal, Write};
use std::os::unix::ffi::OsStrExt;

/// SGR codes per file type, plus `*.ext` overrides (as in LS_COLORS).
/// An empty code means "no coloring".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    dir: String,
    exec: String,
    link: String,
    orphan: String,
    fifo: String,
    sock: String,
    chr: String,
    blk: String,
    image: String,
    video: String,
    audio: String,
    archive: String,
    file: String,
    /// (suffix such as ".png", code), first match wins
    ext: Vec<(Vec<u8>, String)>,
}

impl Palette {
    /// Eka's own scheme, used when LS_COLORS is not set
    pub fn eka() -> Self {
        let c = |s: &str| s.to_string();
        Self {
            dir: c("36"),
            exec: c("32"),
            link: c("33"),
            orphan: c("31;1"),
            fifo: c("33"),
            sock: c("35"),
            chr: c("33;1"),
            blk: c("33;1"),
            image: c("35"),
            video: c("31"),
            audio: c("38;5;208"),
            archive: c("34"),
            file: String::new(),
            ext: Vec::new(),
        }
    }

    /// GNU defaults overlaid with an LS_COLORS string. Media/archive classes
    /// fall back to `fi`; per-extension rules come from `*.ext` keys.
    pub fn from_ls_colors(spec: &str) -> Self {
        let c = |s: &str| s.to_string();
        let mut p = Self {
            dir: c("01;34"),
            exec: c("01;32"),
            link: c("01;36"),
            orphan: c("40;31;01"),
            fifo: c("40;33"),
            sock: c("01;35"),
            chr: c("40;33;01"),
            blk: c("40;33;01"),
            image: String::new(),
            video: String::new(),
            audio: String::new(),
            archive: String::new(),
            file: String::new(),
            ext: Vec::new(),
        };
        for item in spec.split(':') {
            let Some((key, val)) = item.split_once('=') else {
                continue;
            };
            let val = val.to_string();
            match key {
                "di" => p.dir = val,
                "ex" => p.exec = val,
                "ln" => p.link = val,
                "or" => p.orphan = val,
                "pi" => p.fifo = val,
                "so" => p.sock = val,
                "cd" => p.chr = val,
                "bd" => p.blk = val,
                "fi" => p.file = val,
                k => {
                    if let Some(suffix) = k.strip_prefix('*') {
                        p.ext.push((suffix.as_bytes().to_vec(), val));
                    }
                }
            }
        }
        // Classes with no dedicated LS_COLORS key look like plain files
        p.image = p.file.clone();
        p.video = p.file.clone();
        p.audio = p.file.clone();
        p.archive = p.file.clone();
        p
    }

    /// Builds the palette from the environment (LS_COLORS)
    pub fn from_env() -> Self {
        match std::env::var("LS_COLORS") {
            Ok(v) if !v.is_empty() => Self::from_ls_colors(&v),
            _ => Self::eka(),
        }
    }

    /// SGR code for an entry; empty means uncolored
    pub fn code_for(&self, file_type: FileType, name: &[u8]) -> &str {
        let by_type = match file_type {
            FileType::Directory => &self.dir,
            FileType::Executable => &self.exec,
            FileType::Symlink => &self.link,
            FileType::BrokenSymlink => &self.orphan,
            FileType::Fifo => &self.fifo,
            FileType::Socket => &self.sock,
            FileType::CharDevice => &self.chr,
            FileType::BlockDevice => &self.blk,
            FileType::Image => &self.image,
            FileType::Video => &self.video,
            FileType::Audio => &self.audio,
            FileType::Compressed => &self.archive,
            FileType::Regular => &self.file,
        };
        let plain_file = matches!(
            file_type,
            FileType::Regular
                | FileType::Image
                | FileType::Video
                | FileType::Audio
                | FileType::Compressed
        );
        if plain_file
            && let Some((_, code)) = self.ext.iter().find(|(suffix, _)| name.ends_with(suffix))
        {
            return code;
        }
        by_type
    }
}

/// Decides whether output is colored
pub fn color_enabled(when: Option<ColorWhen>) -> bool {
    match when {
        Some(ColorWhen::Always) => true,
        Some(ColorWhen::Never) => false,
        Some(ColorWhen::Auto) | None => {
            let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
            let dumb = std::env::var_os("TERM").is_some_and(|t| t == "dumb");
            !no_color && !dumb && io::stdout().is_terminal()
        }
    }
}

/// Writes file names, colored when a palette is present
pub struct Painter {
    palette: Option<Palette>,
}

impl Painter {
    pub fn new(palette: Option<Palette>) -> Self {
        Self { palette }
    }

    /// Writes `shown` (the already quoted name), wrapped in the entry's
    /// color if any. The color is chosen from the raw name, not from `shown`.
    pub fn write_name(
        &self,
        out: &mut impl Write,
        entry: &FileEntry,
        shown: &[u8],
    ) -> io::Result<()> {
        let code = self
            .palette
            .as_ref()
            .map_or("", |p| p.code_for(entry.file_type, entry.name.as_bytes()));
        if code.is_empty() {
            out.write_all(shown)
        } else {
            write!(out, "\x1b[{}m", code)?;
            out.write_all(shown)?;
            out.write_all(b"\x1b[0m")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_colors_parsing() {
        let p = Palette::from_ls_colors("di=01;35:ln=36:*.rs=33:*.tar.gz=31:bogus:ex=");
        assert_eq!(p.code_for(FileType::Directory, b"d"), "01;35");
        assert_eq!(p.code_for(FileType::Symlink, b"l"), "36");
        assert_eq!(p.code_for(FileType::Regular, b"main.rs"), "33");
        assert_eq!(p.code_for(FileType::Compressed, b"a.tar.gz"), "31");
        // empty value disables the class; unknown key ignored
        assert_eq!(p.code_for(FileType::Executable, b"run"), "");
        assert_eq!(p.code_for(FileType::Regular, b"plain"), "");
    }

    #[test]
    fn test_ls_colors_defaults_when_key_missing() {
        let p = Palette::from_ls_colors("");
        assert_eq!(p.code_for(FileType::Directory, b"d"), "01;34");
        assert_eq!(p.code_for(FileType::BrokenSymlink, b"x"), "40;31;01");
    }

    #[test]
    fn test_ext_does_not_override_directory_or_exec() {
        let p = Palette::from_ls_colors("*.rs=33:di=34:ex=32");
        assert_eq!(p.code_for(FileType::Directory, b"dir.rs"), "34");
        assert_eq!(p.code_for(FileType::Executable, b"run.rs"), "32");
    }

    #[test]
    fn test_color_when_explicit() {
        assert!(color_enabled(Some(ColorWhen::Always)));
        assert!(!color_enabled(Some(ColorWhen::Never)));
    }

    fn entry(name: &str, ft: FileType) -> FileEntry {
        FileEntry {
            name: name.into(),
            path: name.into(),
            file_type: ft,
            size: 0,
            rdev: 0,
            permissions: String::new(),
            modified: std::time::UNIX_EPOCH,
            accessed: std::time::UNIX_EPOCH,
            created: None,
            changed: std::time::UNIX_EPOCH,
            symlink_target: None,
            inode: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            blocks: 0,
            links_to_dir: false,
        }
    }

    #[test]
    fn test_painter_plain_and_colored() {
        let e = entry("dir", FileType::Directory);
        let mut plain = Vec::new();
        Painter::new(None)
            .write_name(&mut plain, &e, e.name.as_bytes())
            .unwrap();
        assert_eq!(plain, b"dir");

        let mut colored = Vec::new();
        Painter::new(Some(Palette::eka()))
            .write_name(&mut colored, &e, e.name.as_bytes())
            .unwrap();
        assert_eq!(colored, b"\x1b[36mdir\x1b[0m");
    }

    #[test]
    fn test_empty_code_emits_no_escapes() {
        let e = entry("plain", FileType::Regular);
        let mut buf = Vec::new();
        Painter::new(Some(Palette::eka()))
            .write_name(&mut buf, &e, e.name.as_bytes())
            .unwrap();
        assert_eq!(buf, b"plain");
    }
}
