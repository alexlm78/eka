use crate::cli::Args;
use crate::colors::{Painter, Palette, color_enabled};
use crate::types::FileEntry;
use chrono::{DateTime, Local};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::time::{Duration, SystemTime};

/// Half a Gregorian year, the cutoff between "Mon DD HH:MM" and "Mon DD  YYYY"
const SIX_MONTHS: Duration = Duration::from_secs(31_556_952 / 2);

/// Output formatter for eka
pub struct Formatter<'a> {
    args: &'a Args,
    painter: Painter,
    now: SystemTime,
}

impl<'a> Formatter<'a> {
    pub fn new(args: &'a Args) -> Self {
        let palette = color_enabled(args.color).then(Palette::from_env);
        Self {
            args,
            painter: Painter::new(palette),
            now: SystemTime::now(),
        }
    }

    /// Prints entries in short format (one per line)
    pub fn format_short(&self, out: &mut impl Write, entries: &[FileEntry]) -> io::Result<()> {
        let inode_w = self.inode_width(entries);
        for entry in entries {
            if self.args.inode {
                write!(out, "{:>inode_w$} ", entry.inode)?;
            }
            self.painter.write_name(out, entry)?;
            self.write_indicator(out, entry)?;
            out.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Prints entries in long format. No header row, columns sized to fit.
    pub fn format_long(&self, out: &mut impl Write, entries: &[FileEntry]) -> io::Result<()> {
        let rows: Vec<LongRow> = entries.iter().map(|e| self.long_row(e)).collect();
        let w = |f: fn(&LongRow) -> usize| rows.iter().map(f).max().unwrap_or(0);
        let (nlink_w, owner_w, group_w, size_w) = (
            w(|r| r.nlink.len()),
            w(|r| r.owner.len()),
            w(|r| r.group.len()),
            w(|r| r.size.len()),
        );
        let inode_w = self.inode_width(entries);

        for (entry, row) in entries.iter().zip(&rows) {
            if self.args.inode {
                write!(out, "{:>inode_w$} ", entry.inode)?;
            }
            write!(
                out,
                "{} {:>nlink_w$} {:<owner_w$} {:<group_w$} {:>size_w$} {} ",
                entry.permissions, row.nlink, row.owner, row.group, row.size, row.time
            )?;
            self.painter.write_name(out, entry)?;
            if let Some(target) = &entry.symlink_target {
                out.write_all(b" -> ")?;
                out.write_all(target.as_bytes())?;
            } else {
                self.write_indicator(out, entry)?;
            }
            out.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Prints the `total N` line (allocated size in 1 KiB units)
    pub fn write_total(&self, out: &mut impl Write, entries: &[FileEntry]) -> io::Result<()> {
        let sectors: u64 = entries.iter().map(|e| e.blocks).sum();
        let kib = sectors.div_ceil(2);
        if self.args.human_readable {
            writeln!(out, "total {}", human_size(kib * 1024))
        } else {
            writeln!(out, "total {}", kib)
        }
    }

    fn inode_width(&self, entries: &[FileEntry]) -> usize {
        if !self.args.inode {
            return 0;
        }
        entries
            .iter()
            .map(|e| e.inode.to_string().len())
            .max()
            .unwrap_or(0)
    }

    fn write_indicator(&self, out: &mut impl Write, entry: &FileEntry) -> io::Result<()> {
        if self.args.classify {
            out.write_all(entry.file_type.indicator().as_bytes())?;
        }
        Ok(())
    }

    fn long_row(&self, entry: &FileEntry) -> LongRow {
        let size = if self.args.human_readable {
            human_size(entry.size)
        } else {
            entry.size.to_string()
        };
        LongRow {
            nlink: entry.nlink.to_string(),
            // Name resolution (getpwuid/getgrgid with a cache) comes in the next phase
            owner: entry.uid.to_string(),
            group: entry.gid.to_string(),
            size,
            time: format_time(entry.time(self.args.time_field()), self.now),
        }
    }
}

struct LongRow {
    nlink: String,
    owner: String,
    group: String,
    size: String,
    time: String,
}

/// Formats a size like `ls -h`: powers of 1024, rounded up, one decimal below 10
pub fn human_size(size: u64) -> String {
    const UNITS: [char; 6] = ['K', 'M', 'G', 'T', 'P', 'E'];
    if size < 1024 {
        return size.to_string();
    }
    let mut unit = 0;
    let mut div: u128 = 1024;
    while unit + 1 < UNITS.len() && (size as u128) >= div * 1024 {
        div *= 1024;
        unit += 1;
    }
    let size = size as u128;
    let tenths = (size * 10).div_ceil(div);
    if tenths < 100 {
        return format!("{}.{}{}", tenths / 10, tenths % 10, UNITS[unit]);
    }
    let whole = size.div_ceil(div);
    if whole >= 1024 && unit + 1 < UNITS.len() {
        return format!("1.0{}", UNITS[unit + 1]);
    }
    format!("{}{}", whole, UNITS[unit])
}

/// `Mon DD HH:MM` for files from the last six months, `Mon DD  YYYY` otherwise
fn format_time(t: SystemTime, now: SystemTime) -> String {
    let recent = now
        .checked_sub(SIX_MONTHS)
        .is_some_and(|cutoff| t >= cutoff)
        && t <= now;
    let dt: DateTime<Local> = t.into();
    if recent {
        dt.format("%b %e %H:%M").to_string()
    } else {
        dt.format("%b %e  %Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::path::PathBuf;

    fn entry(name: &[u8]) -> FileEntry {
        FileEntry {
            name: std::ffi::OsStr::from_bytes(name).to_os_string(),
            path: PathBuf::from("x"),
            file_type: FileType::Regular,
            size: 1024,
            permissions: "-rw-r--r--".to_string(),
            modified: SystemTime::now(),
            accessed: SystemTime::UNIX_EPOCH,
            changed: SystemTime::UNIX_EPOCH,
            is_hidden: name.starts_with(b"."),
            symlink_target: None,
            inode: 12345,
            nlink: 1,
            uid: 501,
            gid: 20,
            blocks: 8,
        }
    }

    fn render(args: &Args, entries: &[FileEntry], long: bool) -> Vec<u8> {
        let args = Args {
            color: Some(crate::cli::ColorWhen::Never),
            ..clone_args(args)
        };
        let f = Formatter::new(&args);
        let mut buf = Vec::new();
        if long {
            f.format_long(&mut buf, entries).unwrap();
        } else {
            f.format_short(&mut buf, entries).unwrap();
        }
        buf
    }

    // Args is not Clone (clap value types only); copy the fields tests vary
    fn clone_args(a: &Args) -> Args {
        Args {
            inode: a.inode,
            classify: a.classify,
            human_readable: a.human_readable,
            long: a.long,
            ..Default::default()
        }
    }

    #[test]
    fn test_short_one_line_per_entry_no_color() {
        let out = render(&Args::default(), &[entry(b"a"), entry(b"b")], false);
        assert_eq!(out, b"a\nb\n");
    }

    #[test]
    fn test_short_inode_and_classify() {
        let mut dir = entry(b"d");
        dir.file_type = FileType::Directory;
        let args = Args {
            inode: true,
            classify: true,
            ..Default::default()
        };
        let out = render(&args, &[dir], false);
        assert_eq!(out, b"12345 d/\n");
    }

    #[test]
    fn test_non_utf8_bytes_written_raw() {
        let out = render(&Args::default(), &[entry(b"bad\xff")], false);
        assert_eq!(out, b"bad\xff\n");
    }

    #[test]
    fn test_long_has_no_header_and_aligned_columns() {
        let mut big = entry(b"big");
        big.size = 1_234_567;
        big.nlink = 12;
        let out = String::from_utf8(render(&Args::default(), &[entry(b"f"), big], true)).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("-rw-r--r--  1 501 20    1024 "));
        assert!(lines[1].starts_with("-rw-r--r-- 12 501 20 1234567 "));
        assert!(lines[0].ends_with(" f") && lines[1].ends_with(" big"));
    }

    #[test]
    fn test_long_symlink_shows_target_not_indicator() {
        let mut l = entry(b"ln");
        l.file_type = FileType::Symlink;
        l.symlink_target = Some("real".into());
        let args = Args {
            classify: true,
            ..Default::default()
        };
        let out = String::from_utf8(render(&args, &[l], true)).unwrap();
        assert!(out.trim_end().ends_with(" ln -> real"));
    }

    #[test]
    fn test_total_in_kib_rounds_up() {
        let mut a = entry(b"a");
        a.blocks = 3; // 1.5 KiB
        let mut b = entry(b"b");
        b.blocks = 4; // 2 KiB
        let args = Args::default();
        let f = Formatter::new(&args);
        let mut buf = Vec::new();
        f.write_total(&mut buf, &[a, b]).unwrap();
        assert_eq!(buf, b"total 4\n"); // ceil(7 / 2)
    }

    #[test]
    fn test_total_empty_dir_is_zero() {
        let args = Args::default();
        let mut buf = Vec::new();
        Formatter::new(&args).write_total(&mut buf, &[]).unwrap();
        assert_eq!(buf, b"total 0\n");
    }

    #[test]
    fn test_human_size() {
        assert_eq!(human_size(0), "0");
        assert_eq!(human_size(1023), "1023");
        assert_eq!(human_size(1024), "1.0K");
        assert_eq!(human_size(1025), "1.1K"); // rounds up like GNU
        assert_eq!(human_size(1536), "1.5K");
        assert_eq!(human_size(10 * 1024), "10K");
        assert_eq!(human_size(10 * 1024 + 1), "11K");
        assert_eq!(human_size(1024 * 1024 - 1), "1.0M"); // 1024K rolls over
        assert_eq!(human_size(5 * 1024 * 1024 * 1024), "5.0G");
        assert_eq!(human_size(u64::MAX), "16E");
    }

    #[test]
    fn test_time_format_recent_vs_old() {
        let now = SystemTime::now();
        let recent = format_time(now - Duration::from_secs(86_400), now);
        assert!(recent.contains(':'), "{recent}");
        let old = format_time(now - Duration::from_secs(400 * 86_400), now);
        assert!(!old.contains(':') && old.contains("  "), "{old}");
        // future timestamps show the year as well
        let future = format_time(now + Duration::from_secs(86_400 * 30), now);
        assert!(!future.contains(':'), "{future}");
    }
}
