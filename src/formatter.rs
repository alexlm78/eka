use crate::cli::Args;
use crate::colors::{ColoredOutput, Colors};
use crate::types::FileEntry;
use chrono::{DateTime, Local};
use std::io::{self, Write};

/// Output formatter for eka
pub struct Formatter;

impl Formatter {
    /// Prints entries in short format (one per line)
    pub fn format_short(out: &mut impl Write, entries: &[FileEntry], args: &Args) -> io::Result<()> {
        for entry in entries {
            if args.inode {
                write!(out, "{:>12} ", entry.inode)?;
            }
            Colors::write_filename(out, entry, args.classify)?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Prints entries in long format (with columns)
    pub fn format_long(out: &mut impl Write, entries: &[FileEntry], args: &Args) -> io::Result<()> {
        let size_col = if args.human_readable { "  Size" } else { "      Size" };
        writeln!(out, "Permissions    Links User     Group {} Date                Name", size_col)?;
        for entry in entries {
            Self::write_long_entry(out, entry, args)?;
        }
        Ok(())
    }

    /// Prints a single entry in long format
    fn write_long_entry(out: &mut impl Write, entry: &FileEntry, args: &Args) -> io::Result<()> {
        if args.inode {
            write!(out, "{:>12} ", entry.inode)?;
        }

        ColoredOutput::write_permissions(out, &entry.permissions, entry.file_type.is_dir())?;
        write!(out, "  {:>4} ", entry.nlink)?;

        // User and group (simplified; real name resolution is Phase 2)
        write!(out, "{:<8} {:<8} ", "user", "staff")?;

        if args.human_readable {
            ColoredOutput::write_human_size(out, entry.size)?;
            out.write_all(b" ")?;
        } else {
            write!(out, "{:>10} ", entry.size)?;
        }

        let datetime: DateTime<Local> = entry.modified.into();
        write!(out, "{:>20} ", datetime.format("%b %d %H:%M"))?;

        Colors::write_filename(out, entry, args.classify)?;
        out.write_all(b"\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn entry(name: &[u8]) -> FileEntry {
        FileEntry {
            name: std::ffi::OsStr::from_bytes(name).to_os_string(),
            path: PathBuf::from("x"),
            file_type: FileType::Regular,
            size: 1024,
            permissions: "-rw-r--r--".to_string(),
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            changed: SystemTime::UNIX_EPOCH,
            is_hidden: name.starts_with(b"."),
            is_symlink: false,
            symlink_target: None,
            inode: 12345,
            nlink: 1,
            uid: 0,
            gid: 0,
            blocks: 8,
        }
    }

    #[test]
    fn test_short_writes_one_line_per_entry() {
        let mut buf = Vec::new();
        Formatter::format_short(&mut buf, &[entry(b"a"), entry(b"b")], &Args::default()).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert_eq!(s.lines().count(), 2);
        assert!(s.contains('a') && s.contains('b'));
    }

    #[test]
    fn test_short_inode() {
        let mut buf = Vec::new();
        let args = Args { inode: true, ..Default::default() };
        Formatter::format_short(&mut buf, &[entry(b"a")], &args).unwrap();
        assert!(String::from_utf8(buf).unwrap().contains("12345"));
    }

    #[test]
    fn test_non_utf8_bytes_written_raw() {
        let mut buf = Vec::new();
        Formatter::format_short(&mut buf, &[entry(b"bad\xff")], &Args::default()).unwrap();
        assert!(buf.windows(4).any(|w| w == b"bad\xff"));
    }

    #[test]
    fn test_long_has_permissions_and_size() {
        let mut buf = Vec::new();
        Formatter::format_long(&mut buf, &[entry(b"f")], &Args::default()).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("-rw-r--r--") && s.contains("1024"));
    }
}
