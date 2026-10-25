use crate::cli::{Args, IndicatorStyle, Layout, TimeStyle};
use crate::colors::{Painter, Palette, color_enabled};
use crate::layout;
use crate::quoting::Quoter;
use crate::types::{FileEntry, FileType, device_numbers};
use chrono::{DateTime, Local, Timelike};
use std::cell::Cell;
use std::fmt::Write as _;
use std::io::{self, IsTerminal, Write};
use std::os::unix::ffi::OsStrExt;
use std::time::{Duration, SystemTime};
use uzers::{Groups, Users, UsersCache};

/// Half a Gregorian year, the cutoff between "Mon DD HH:MM" and "Mon DD  YYYY"
const SIX_MONTHS: Duration = Duration::from_secs(31_556_952 / 2);

/// Output formatter for eka
pub struct Formatter<'a> {
    args: &'a Args,
    painter: Painter,
    now: SystemTime,
    layout: Layout,
    width: usize,
    /// Caches uid/gid lookups for the lifetime of the run
    users: UsersCache,
    quoter: Quoter,
    /// Tab stop distance for column padding; 0 pads with spaces only
    tab_size: usize,
    /// Set per listing: some name is quoted, so unquoted ones get a leading space
    pad_unquoted: Cell<bool>,
}

impl<'a> Formatter<'a> {
    pub fn new(args: &'a Args) -> Self {
        let palette = color_enabled(args.color).then(Palette::from_env);
        let tty = io::stdout().is_terminal();
        let layout = args.layout(tty);
        // Only column layouts need the terminal width
        let width = match layout {
            Layout::Columns | Layout::Across | Layout::Commas => args.line_width(),
            _ => 0,
        };
        Self {
            args,
            painter: Painter::new(palette),
            now: SystemTime::now(),
            layout,
            width,
            users: UsersCache::new(),
            quoter: Quoter::new(args.quoting_style(tty), args.hide_control(tty)),
            // GNU ls drops the tabs when the width is unlimited
            tab_size: if width == 0 { 0 } else { args.tab_size() },
            pad_unquoted: Cell::new(false),
        }
    }

    /// Prints entries in the short format selected by -1, -C, -x or -m
    pub fn format_short(&self, out: &mut impl Write, entries: &[FileEntry]) -> io::Result<()> {
        let inode_w = self.inode_width(entries);
        let size_w = self.block_width(entries);
        // GNU ls pads unquoted names in the column and long layouts only
        self.set_alignment(
            entries,
            matches!(self.layout, Layout::Columns | Layout::Across),
        );
        match self.layout {
            Layout::Columns => self.write_grid(out, entries, inode_w, size_w, false),
            Layout::Across => self.write_grid(out, entries, inode_w, size_w, true),
            Layout::Commas => self.write_commas(out, entries, inode_w, size_w),
            Layout::Single | Layout::Long => {
                for entry in entries {
                    self.write_cell(out, entry, inode_w, size_w)?;
                    out.write_all(b"\n")?;
                }
                Ok(())
            }
        }
    }

    fn write_grid(
        &self,
        out: &mut impl Write,
        entries: &[FileEntry],
        inode_w: usize,
        size_w: usize,
        across: bool,
    ) -> io::Result<()> {
        let widths: Vec<usize> = entries
            .iter()
            .map(|e| self.cell_width(e, inode_w, size_w))
            .collect();
        let plan = layout::plan(&widths, self.width, across);
        for row in 0..plan.rows {
            // Column where the current cell starts
            let mut start = 0;
            for col in 0..plan.cols() {
                let Some(i) = plan.index(row, col) else {
                    continue;
                };
                self.write_cell(out, &entries[i], inode_w, size_w)?;
                // Pad only when another cell follows on this row
                if plan.index(row, col + 1).is_some() {
                    let next = start + plan.col_widths[col] + 2;
                    indent(out, start + widths[i], next, self.tab_size)?;
                    start = next;
                }
            }
            out.write_all(b"\n")?;
        }
        Ok(())
    }

    fn write_commas(
        &self,
        out: &mut impl Write,
        entries: &[FileEntry],
        inode_w: usize,
        size_w: usize,
    ) -> io::Result<()> {
        let mut pos = 0;
        for (i, entry) in entries.iter().enumerate() {
            let w = self.cell_width(entry, inode_w, size_w);
            if i > 0 {
                if layout::comma_fits(pos, w, self.width) {
                    out.write_all(b", ")?;
                    pos += 2;
                } else {
                    out.write_all(b",\n")?;
                    pos = 0;
                }
            }
            self.write_cell(out, entry, inode_w, size_w)?;
            pos += w;
        }
        if !entries.is_empty() {
            out.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Writes inode and allocated size (if asked), colored name and indicator
    fn write_cell(
        &self,
        out: &mut impl Write,
        entry: &FileEntry,
        inode_w: usize,
        size_w: usize,
    ) -> io::Result<()> {
        if self.args.inode {
            write!(out, "{:>inode_w$} ", entry.inode)?;
        }
        if self.args.size {
            write!(out, "{:>size_w$} ", self.allocated_size(entry))?;
        }
        self.write_name(out, entry)?;
        self.write_indicator(out, entry)
    }

    /// Display width of the cell `write_cell` produces
    fn cell_width(&self, entry: &FileEntry, inode_w: usize, size_w: usize) -> usize {
        let inode = if self.args.inode { inode_w + 1 } else { 0 };
        let size = if self.args.size { size_w + 1 } else { 0 };
        inode + size + self.name_cells(entry) + self.indicator(entry).len()
    }

    /// Records whether this listing mixes quoted and unquoted names
    fn set_alignment(&self, entries: &[FileEntry], enabled: bool) {
        let mixed = enabled
            && self.quoter.aligns()
            && entries
                .iter()
                .any(|e| self.quoter.is_quoted(e.name.as_bytes()));
        self.pad_unquoted.set(mixed);
    }

    /// Whether this name gets the alignment space
    fn needs_pad(&self, name: &[u8]) -> bool {
        self.pad_unquoted.get() && !self.quoter.is_quoted(name)
    }

    /// Display columns of the quoted name, with its alignment space
    fn name_cells(&self, entry: &FileEntry) -> usize {
        let name = entry.name.as_bytes();
        name_width(&self.quoter.quote(name)) + usize::from(self.needs_pad(name))
    }

    /// Writes the quoted, colored name preceded by the alignment space if needed
    fn write_name(&self, out: &mut impl Write, entry: &FileEntry) -> io::Result<()> {
        let name = entry.name.as_bytes();
        if self.needs_pad(name) {
            out.write_all(b" ")?;
        }
        self.painter
            .write_name(out, entry, &self.quoter.quote(name))
    }

    /// Writes a directory heading such as `src:` with the path quoted
    pub fn write_heading(&self, out: &mut impl Write, path: &std::path::Path) -> io::Result<()> {
        out.write_all(&self.quoter.quote_heading(path.as_os_str().as_bytes()))?;
        out.write_all(b":\n")
    }

    /// Prints entries in long format. No header row, columns sized to fit.
    pub fn format_long(&self, out: &mut impl Write, entries: &[FileEntry]) -> io::Result<()> {
        let rows: Vec<LongRow> = entries.iter().map(|e| self.long_row(e)).collect();
        let w = |f: fn(&LongRow) -> usize| rows.iter().map(f).max().unwrap_or(0);
        let (nlink_w, owner_w, group_w, size_w, block_w) = (
            w(|r| r.nlink.len()),
            w(|r| name_width(r.owner.as_bytes())),
            w(|r| name_width(r.group.as_bytes())),
            w(|r| r.size.len()),
            entries
                .iter()
                .map(|entry| self.allocated_size(entry).len())
                .max()
                .unwrap_or(0),
        );
        let inode_w = self.inode_width(entries);
        self.set_alignment(entries, true);

        // The marker column exists only when some entry has a marker, so a
        // listing without extended attributes or ACLs is unchanged
        let markers: Vec<&str> = entries
            .iter()
            .map(|e| crate::attrs::marker(&e.path))
            .collect();
        let marker_column = markers.iter().any(|m| !m.is_empty());

        for ((entry, row), marker) in entries.iter().zip(&rows).zip(&markers) {
            if self.args.inode {
                write!(out, "{:>inode_w$} ", entry.inode)?;
            }
            if self.args.size {
                write!(out, "{:>block_w$} ", self.allocated_size(entry))?;
            }
            let marker = if marker_column && marker.is_empty() {
                " "
            } else {
                marker
            };
            write!(
                out,
                "{}{} {:>nlink_w$}",
                entry.permissions, marker, row.nlink
            )?;
            if !self.args.no_owner {
                write_padded(out, &row.owner, owner_w)?;
            }
            if !self.args.no_group_short && !self.args.no_group {
                write_padded(out, &row.group, group_w)?;
            }
            write!(out, " {:>size_w$} {} ", row.size, row.time)?;
            self.write_name(out, entry)?;
            if let Some(target) = &entry.symlink_target {
                out.write_all(b" -> ")?;
                out.write_all(&self.quoter.quote(target.as_bytes()))?;
            } else {
                self.write_indicator(out, entry)?;
            }
            out.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Prints the `total N` line in the selected block size
    pub fn write_total(&self, out: &mut impl Write, entries: &[FileEntry]) -> io::Result<()> {
        let bytes: u64 = entries.iter().map(|e| e.blocks.saturating_mul(512)).sum();
        let blocks = bytes.div_ceil(self.display_block_size());
        if self.args.human() {
            writeln!(out, "total {}", human_size(bytes, self.args.si))
        } else {
            writeln!(out, "total {}{}", blocks, self.block_suffix())
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

    fn block_width(&self, entries: &[FileEntry]) -> usize {
        if !self.args.size {
            return 0;
        }
        entries
            .iter()
            .map(|entry| self.allocated_size(entry).len())
            .max()
            .unwrap_or(0)
    }

    fn display_block_size(&self) -> u64 {
        self.args.block_size.map_or(1024, |b| b.bytes)
    }

    /// Suffix printed after scaled sizes (`--block-size=K` prints `16K`)
    fn block_suffix(&self) -> &'static str {
        self.args.block_size.map_or("", |b| b.suffix)
    }

    fn allocated_size(&self, entry: &FileEntry) -> String {
        let bytes = entry.blocks.saturating_mul(512);
        if self.args.human() {
            human_size(bytes, self.args.si)
        } else {
            format!(
                "{}{}",
                bytes.div_ceil(self.display_block_size()),
                self.block_suffix()
            )
        }
    }

    fn write_indicator(&self, out: &mut impl Write, entry: &FileEntry) -> io::Result<()> {
        out.write_all(self.indicator(entry).as_bytes())
    }

    /// Indicator appended to an entry name under the selected style
    fn indicator(&self, entry: &FileEntry) -> &'static str {
        match self.args.indicator() {
            IndicatorStyle::None => "",
            IndicatorStyle::Slash if entry.file_type.is_dir() => "/",
            IndicatorStyle::Slash => "",
            IndicatorStyle::FileType => match entry.file_type.indicator() {
                "*" => "",
                other => other,
            },
            IndicatorStyle::Classify => entry.file_type.indicator(),
        }
    }

    fn long_row(&self, entry: &FileEntry) -> LongRow {
        let size = if matches!(
            entry.file_type,
            FileType::CharDevice | FileType::BlockDevice
        ) {
            let (major, minor) = device_numbers(entry.rdev);
            format!("{major}, {minor}")
        } else if self.args.human() {
            human_size(entry.size, self.args.si)
        } else if let Some(block_size) = self.args.block_size {
            format!(
                "{}{}",
                entry.size.div_ceil(block_size.bytes),
                block_size.suffix
            )
        } else {
            entry.size.to_string()
        };
        LongRow {
            nlink: entry.nlink.to_string(),
            owner: self.user_name(entry.uid),
            group: self.group_name(entry.gid),
            size,
            time: if entry.has_time(self.args.time_field()) {
                format_time(
                    entry.time(self.args.time_field()),
                    self.now,
                    &self.args.time_style(),
                )
            } else {
                "?".to_string()
            },
        }
    }

    /// Owner name, or the numeric id when it has no entry
    fn user_name(&self, uid: u32) -> String {
        if self.args.numeric_uid_gid {
            return uid.to_string();
        }
        self.users.get_user_by_uid(uid).map_or_else(
            || uid.to_string(),
            |u| u.name().to_string_lossy().into_owned(),
        )
    }

    /// Group name, or the numeric id when it has no entry
    fn group_name(&self, gid: u32) -> String {
        if self.args.numeric_uid_gid {
            return gid.to_string();
        }
        self.users.get_group_by_gid(gid).map_or_else(
            || gid.to_string(),
            |g| g.name().to_string_lossy().into_owned(),
        )
    }
}

fn write_padded(out: &mut impl Write, value: &str, width: usize) -> io::Result<()> {
    let pad = width - name_width(value.as_bytes());
    write!(out, " {value}{:pad$}", "")
}

struct LongRow {
    nlink: String,
    owner: String,
    group: String,
    size: String,
    time: String,
}

/// Moves from column `from` to column `to` with tabs where a tab stop is
/// crossed and spaces otherwise, the way GNU ls pads its columns
fn indent(out: &mut impl Write, mut from: usize, to: usize, tab_size: usize) -> io::Result<()> {
    while from < to {
        if tab_size != 0 && to / tab_size > (from + 1) / tab_size {
            out.write_all(b"\t")?;
            from += tab_size - from % tab_size;
        } else {
            out.write_all(b" ")?;
            from += 1;
        }
    }
    Ok(())
}

/// Display columns of a name; invalid UTF-8 bytes count as one column each
fn name_width(name: &[u8]) -> usize {
    String::from_utf8_lossy(name).chars().count()
}

/// Formats a size like `ls -h` (powers of 1024) or `ls --si` (powers of 1000):
/// rounded up, one decimal below 10
pub fn human_size(size: u64, si: bool) -> String {
    let units: [char; 6] = [if si { 'k' } else { 'K' }, 'M', 'G', 'T', 'P', 'E'];
    let base: u128 = if si { 1000 } else { 1024 };
    if (size as u128) < base {
        return size.to_string();
    }
    let mut unit = 0;
    let mut div: u128 = base;
    while unit + 1 < units.len() && (size as u128) >= div * base {
        div *= base;
        unit += 1;
    }
    let size = size as u128;
    let tenths = (size * 10).div_ceil(div);
    if tenths < 100 {
        return format!("{}.{}{}", tenths / 10, tenths % 10, units[unit]);
    }
    let whole = size.div_ceil(div);
    if whole >= base && unit + 1 < units.len() {
        return format!("1.0{}", units[unit + 1]);
    }
    format!("{}{}", whole, units[unit])
}

/// Converts to local time without chrono's panicking `From<SystemTime>`, which
/// fails outside roughly +-262,000 years. None when the time is unrepresentable.
fn to_local(t: SystemTime) -> Option<DateTime<Local>> {
    let (secs, nanos) = match t.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => (i64::try_from(d.as_secs()).ok()?, d.subsec_nanos()),
        Err(e) => {
            let d = e.duration();
            let secs = i64::try_from(d.as_secs()).ok()?.checked_neg()?;
            match d.subsec_nanos() {
                0 => (secs, 0),
                n => (secs.checked_sub(1)?, 1_000_000_000 - n),
            }
        }
    };
    DateTime::from_timestamp(secs, nanos).map(|utc| utc.with_timezone(&Local))
}

/// Formats timestamps using GNU ls's recent-file rule.
pub fn format_time(t: SystemTime, now: SystemTime, style: &TimeStyle) -> String {
    let recent = now
        .checked_sub(SIX_MONTHS)
        .is_some_and(|cutoff| t >= cutoff)
        && t <= now;
    let Some(dt) = to_local(t) else {
        return "?".to_string();
    };
    match style {
        TimeStyle::Locale => {
            if recent {
                dt.format("%b %e %H:%M").to_string()
            } else {
                dt.format("%b %e  %Y").to_string()
            }
        }
        TimeStyle::Iso => {
            if recent {
                dt.format("%m-%d %H:%M").to_string()
            } else {
                dt.format("%Y-%m-%d ").to_string()
            }
        }
        TimeStyle::LongIso => dt.format("%Y-%m-%d %H:%M").to_string(),
        TimeStyle::FullIso => format!(
            "{}.{:09} {}",
            dt.format("%Y-%m-%d %H:%M:%S"),
            dt.nanosecond(),
            dt.format("%z")
        ),
        TimeStyle::Custom(format) => {
            let format = match (recent, format.split_once('\n')) {
                (true, Some((_, recent))) => recent,
                (false, Some((old, _))) => old,
                (_, None) => format,
            };
            let mut shown = String::new();
            match write!(shown, "{}", dt.format(format)) {
                Ok(()) => shown,
                // Validated when parsed; never panic on a format chrono rejects
                Err(_) => "?".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::path::PathBuf;

    /// An id with no passwd/group entry, so names fall back to numbers
    const NO_USER: u32 = 4_000_000_001;

    fn entry(name: &[u8]) -> FileEntry {
        FileEntry {
            name: std::ffi::OsStr::from_bytes(name).to_os_string(),
            path: PathBuf::from("x"),
            file_type: FileType::Regular,
            size: 1024,
            rdev: 0,
            permissions: "-rw-r--r--".to_string(),
            modified: SystemTime::now(),
            accessed: SystemTime::UNIX_EPOCH,
            created: None,
            changed: SystemTime::UNIX_EPOCH,
            symlink_target: None,
            inode: 12345,
            nlink: 1,
            uid: NO_USER,
            gid: NO_USER,
            blocks: 8,
            links_to_dir: false,
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
            numeric_uid_gid: a.numeric_uid_gid,
            no_owner: a.no_owner,
            no_group_short: a.no_group_short,
            no_group: a.no_group,
            size: a.size,
            block_size: a.block_size,
            indicator_style: a.indicator_style,
            time: a.time,
            time_style: a.time_style.clone(),
            full_time: a.full_time,
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
        assert!(lines[0].starts_with("-rw-r--r--  1 4000000001 4000000001    1024 "));
        assert!(lines[1].starts_with("-rw-r--r-- 12 4000000001 4000000001 1234567 "));
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
    fn test_human_size_si() {
        assert_eq!(human_size(999, true), "999");
        assert_eq!(human_size(1000, true), "1.0k");
        assert_eq!(human_size(1500, true), "1.5k");
        assert_eq!(human_size(1_500_000, true), "1.5M");
        assert_eq!(human_size(999_999, true), "1.0M"); // 1000k rolls over
    }

    #[test]
    fn test_human_size() {
        assert_eq!(human_size(0, false), "0");
        assert_eq!(human_size(1023, false), "1023");
        assert_eq!(human_size(1024, false), "1.0K");
        assert_eq!(human_size(1025, false), "1.1K"); // rounds up like GNU
        assert_eq!(human_size(1536, false), "1.5K");
        assert_eq!(human_size(10 * 1024, false), "10K");
        assert_eq!(human_size(10 * 1024 + 1, false), "11K");
        assert_eq!(human_size(1024 * 1024 - 1, false), "1.0M"); // 1024K rolls over
        assert_eq!(human_size(5 * 1024 * 1024 * 1024, false), "5.0G");
        assert_eq!(human_size(u64::MAX, false), "16E");
    }

    #[test]
    fn test_time_format_recent_vs_old() {
        let now = SystemTime::now();
        let recent = format_time(now - Duration::from_secs(86_400), now, &TimeStyle::Locale);
        assert!(recent.contains(':'), "{recent}");
        let old = format_time(
            now - Duration::from_secs(400 * 86_400),
            now,
            &TimeStyle::Locale,
        );
        assert!(!old.contains(':') && old.contains("  "), "{old}");
        // future timestamps show the year as well
        let future = format_time(
            now + Duration::from_secs(86_400 * 30),
            now,
            &TimeStyle::Locale,
        );
        assert!(!future.contains(':'), "{future}");
    }

    fn render_with(args: Args, entries: &[FileEntry]) -> String {
        let args = Args {
            color: Some(crate::cli::ColorWhen::Never),
            ..args
        };
        let mut buf = Vec::new();
        Formatter::new(&args)
            .format_short(&mut buf, entries)
            .unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn names(list: &[&str]) -> Vec<FileEntry> {
        list.iter().map(|n| entry(n.as_bytes())).collect()
    }

    #[test]
    fn test_columns_fill_down_then_across() {
        let args = Args {
            columns: true,
            width: Some(12),
            tabsize: Some(0),
            ..Default::default()
        };
        let out = render_with(args, &names(&["aa", "bb", "cc", "dd", "ee"]));
        // 3 columns of width 2 + 2 gaps = 10 <= 12
        assert_eq!(out, "aa  cc  ee\nbb  dd\n");
    }

    #[test]
    fn test_across_fills_rows_first() {
        let args = Args {
            across: true,
            width: Some(12),
            tabsize: Some(0),
            ..Default::default()
        };
        let out = render_with(args, &names(&["aa", "bb", "cc", "dd", "ee"]));
        assert_eq!(out, "aa  bb  cc\ndd  ee\n");
    }

    #[test]
    fn test_columns_pad_to_widest_in_column_without_trailing_space() {
        let args = Args {
            columns: true,
            width: Some(12),
            tabsize: Some(0),
            ..Default::default()
        };
        let out = render_with(args, &names(&["a", "longname", "b", "c"]));
        assert_eq!(out, "a         b\nlongname  c\n");
        assert!(out.lines().all(|l| !l.ends_with(' ')));
    }

    #[test]
    fn test_columns_with_inode_and_classify_count_in_width() {
        let mut d = entry(b"d");
        d.file_type = FileType::Directory;
        let args = Args {
            columns: true,
            inode: true,
            classify: true,
            width: Some(40),
            ..Default::default()
        };
        let out = render_with(args, &[d, entry(b"f")]);
        assert_eq!(out, "12345 d/  12345 f\n");
    }

    #[test]
    fn test_one_per_line_overrides_when_not_tty() {
        let out = render_with(Args::default(), &names(&["a", "b"]));
        assert_eq!(out, "a\nb\n");
    }

    #[test]
    fn test_commas_wrap_at_width() {
        let args = Args {
            commas: true,
            width: Some(14),
            ..Default::default()
        };
        let out = render_with(args, &names(&["aaaa", "bbbb", "cccc", "dddd"]));
        assert_eq!(out, "aaaa, bbbb,\ncccc, dddd\n");
    }

    #[test]
    fn test_commas_single_entry_and_empty() {
        let args = || Args {
            commas: true,
            width: Some(80),
            ..Default::default()
        };
        assert_eq!(render_with(args(), &names(&["only"])), "only\n");
        assert_eq!(render_with(args(), &[]), "");
    }

    #[test]
    fn test_user_and_group_names_resolved() {
        // uid 0 / gid 0 exist on every Unix; the name is "root" (or "wheel" for gid 0 on macOS)
        let args = Args::default();
        let f = Formatter::new(&args);
        assert_eq!(f.user_name(0), "root");
        assert!(["root", "wheel"].contains(&f.group_name(0).as_str()));
        assert_eq!(f.user_name(NO_USER), NO_USER.to_string());
    }

    #[test]
    fn test_numeric_and_omitted_long_columns() {
        let args = Args {
            numeric_uid_gid: true,
            no_owner: true,
            no_group: true,
            ..Default::default()
        };
        let out = String::from_utf8(render(&args, &[entry(b"f")], true)).unwrap();
        assert!(out.starts_with("-rw-r--r-- 1 1024 "));
        assert!(!out.contains(&NO_USER.to_string()));
        assert!(!out.split(" f").next().unwrap().contains("  "));

        let numeric = Args {
            numeric_uid_gid: true,
            ..Default::default()
        };
        let out = String::from_utf8(render(&numeric, &[entry(b"f")], true)).unwrap();
        assert!(out.contains(&format!(" {NO_USER} {NO_USER} ")));
    }

    #[test]
    fn test_size_column_and_slash_indicator() {
        let mut dir = entry(b"d");
        dir.file_type = FileType::Directory;
        dir.blocks = 3;
        let args = Args {
            inode: true,
            size: true,
            indicator_style: Some(IndicatorStyle::Slash),
            ..Default::default()
        };
        assert_eq!(render(&args, &[dir.clone()], false), b"12345 2 d/\n");

        let classified = Args {
            classify: true,
            ..Default::default()
        };
        assert_eq!(render(&classified, &[dir], false), b"d/\n");

        let mut executable = entry(b"x");
        executable.file_type = FileType::Executable;
        assert_eq!(render(&classified, &[executable], false), b"x*\n");
    }

    #[test]
    fn test_block_size_scales_all_size_outputs() {
        let mut item = entry(b"f");
        item.size = 1025;
        item.blocks = 3;
        let args = Args {
            size: true,
            block_size: Some(crate::cli::BlockSize::of(1000)),
            ..Default::default()
        };
        assert_eq!(render(&args, &[item.clone()], false), b"2 f\n");
        assert!(
            String::from_utf8(render(&args, &[item.clone()], true))
                .unwrap()
                .starts_with("2 -rw-r--r-- 1 4000000001 4000000001 2 ")
        );
        let f = Formatter::new(&args);
        let mut out = Vec::new();
        f.write_total(&mut out, &[item]).unwrap();
        assert_eq!(out, b"total 2\n");
    }

    #[test]
    fn test_long_size_column_goes_before_permissions_after_inode() {
        let mut a = entry(b"a");
        a.blocks = 8; // 4 KiB
        let mut b = entry(b"b");
        b.blocks = 20; // 10 KiB
        let args = Args {
            size: true,
            inode: true,
            ..Default::default()
        };
        let out = String::from_utf8(render(&args, &[a, b], true)).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].starts_with("12345  4 -rw-r--r-- "), "{}", lines[0]);
        assert!(lines[1].starts_with("12345 10 -rw-r--r-- "), "{}", lines[1]);
    }

    #[test]
    fn test_long_without_owner_or_group_has_single_spaces() {
        let item = entry(b"f");
        let no_owner = Args {
            no_owner: true,
            ..Default::default()
        };
        let out = String::from_utf8(render(&no_owner, std::slice::from_ref(&item), true)).unwrap();
        assert!(out.starts_with("-rw-r--r-- 1 4000000001 1024 "), "{out}");
        let neither = Args {
            no_owner: true,
            no_group: true,
            ..Default::default()
        };
        let out = String::from_utf8(render(&neither, &[item], true)).unwrap();
        assert!(out.starts_with("-rw-r--r-- 1 1024 "), "{out}");
    }

    #[test]
    fn test_human_readable_overrides_block_size_for_allocated_size() {
        let mut item = entry(b"f");
        item.blocks = 3;
        let args = Args {
            size: true,
            human_readable: true,
            block_size: Some(crate::cli::BlockSize::of(1)),
            ..Default::default()
        };
        assert_eq!(render(&args, &[item.clone()], false), b"1.5K f\n");
        let mut out = Vec::new();
        Formatter::new(&args)
            .write_total(&mut out, &[item])
            .unwrap();
        assert_eq!(out, b"total 1.5K\n");
    }

    #[test]
    fn test_time_styles_select_the_expected_branch() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000);
        let recent = now - Duration::from_secs(86_400);
        let old = now - SIX_MONTHS - Duration::from_secs(1);
        assert!(format_time(recent, now, &TimeStyle::Iso).contains(':'));
        assert!(format_time(old, now, &TimeStyle::Iso).ends_with(' '));
        assert_eq!(
            format_time(recent, now, &TimeStyle::Custom("old\nrecent".to_string())),
            "recent"
        );
        assert_eq!(
            format_time(old, now, &TimeStyle::Custom("old\nrecent".to_string())),
            "old"
        );
        assert!(format_time(recent, now, &TimeStyle::FullIso).contains('.'));
    }

    use crate::quoting::QuotingStyle;

    fn quoting_args(style: QuotingStyle) -> Args {
        Args {
            quoting_style: Some(style),
            ..Default::default()
        }
    }

    #[test]
    fn test_shell_styles_pad_unquoted_names_only_in_column_layouts() {
        let cols = |style| Args {
            columns: true,
            width: Some(80),
            tabsize: Some(0),
            ..quoting_args(style)
        };
        let list = names(&["a b", "plain"]);
        // columns: " plain" carries a leading space so names line up
        assert_eq!(
            render_with(cols(QuotingStyle::ShellEscape), &list),
            "'a b'   plain\n"
        );
        // nothing quoted: no padding
        assert_eq!(
            render_with(cols(QuotingStyle::ShellEscape), &names(&["x", "y"])),
            "x  y\n"
        );
        // always-quoting styles never pad
        assert_eq!(
            render_with(cols(QuotingStyle::ShellEscapeAlways), &list),
            "'a b'  'plain'\n"
        );
        // one per line and comma layouts never pad (GNU ls does not either)
        assert_eq!(
            render_with(quoting_args(QuotingStyle::ShellEscape), &list),
            "'a b'\nplain\n"
        );
        let commas = Args {
            commas: true,
            width: Some(80),
            ..quoting_args(QuotingStyle::ShellEscape)
        };
        assert_eq!(render_with(commas, &list), "'a b', plain\n");
    }

    #[test]
    fn test_long_format_pads_unquoted_names_when_others_are_quoted() {
        let args = Args {
            color: Some(crate::cli::ColorWhen::Never),
            long: true,
            ..quoting_args(QuotingStyle::ShellEscape)
        };
        let render_long = |list: &[&str]| {
            let mut buf = Vec::new();
            Formatter::new(&args)
                .format_long(&mut buf, &names(list))
                .unwrap();
            String::from_utf8(buf).unwrap()
        };
        let out = render_long(&["a b", "plain"]);
        let rows: Vec<&str> = out.lines().collect();
        assert!(rows[0].ends_with(" 'a b'"), "{out}");
        assert!(
            rows[1].ends_with("  plain"),
            "extra space before the name: {out}"
        );
        // nothing quoted: no extra space
        let out = render_long(&["x", "y"]);
        assert!(
            out.lines()
                .all(|l| !l.ends_with("  x") && !l.ends_with("  y")),
            "{out}"
        );
    }

    #[test]
    fn test_unlimited_width_uses_no_tabs() {
        let args = Args {
            columns: true,
            width: Some(0),
            ..Default::default()
        };
        // "dd" would end before a tab stop at the default tab size
        let out = render_with(args, &names(&["a", "bb", "ccc", "dd", "e", "ffff"]));
        assert!(!out.contains('\t'), "{out:?}");
    }

    #[test]
    fn test_indent_uses_tabs_only_when_a_tab_stop_is_crossed() {
        let pad = |from, to, tab| {
            let mut out = Vec::new();
            indent(&mut out, from, to, tab).unwrap();
            String::from_utf8(out).unwrap()
        };
        assert_eq!(pad(1, 3, 8), "  "); // no stop between
        assert_eq!(pad(1, 10, 8), "\t  "); // 1 -> 8 by tab, then two spaces
        assert_eq!(pad(5, 9, 8), "\t "); // crosses 8
        assert_eq!(pad(7, 9, 8), "  "); // moving 1 column over a stop: spaces
        assert_eq!(pad(0, 16, 8), "\t\t");
        assert_eq!(pad(3, 3, 8), "");
        assert_eq!(pad(1, 10, 0), "         "); // -T 0: spaces only
        assert_eq!(pad(1, 10, 4), "\t\t  "); // 1 -> 4 -> 8, then two spaces to 10
    }

    #[test]
    fn test_columns_pad_with_tabs_like_gnu_ls() {
        let args = Args {
            columns: true,
            width: Some(80),
            ..Default::default()
        };
        // "aaaaaa" ends at column 6 and the next column starts at 8: one tab.
        // The following gaps do not cross a stop and use spaces.
        let out = render_with(args, &names(&["aaaaaa", "b", "c", "d"]));
        assert_eq!(out, "aaaaaa\tb  c  d\n");
        // -T 0 turns tabs off
        let spaces = Args {
            columns: true,
            width: Some(80),
            tabsize: Some(0),
            ..Default::default()
        };
        let out = render_with(spaces, &names(&["aaaaaa", "b", "c", "d"]));
        assert_eq!(out, "aaaaaa  b  c  d\n");
    }

    #[test]
    fn test_hostile_names_are_escaped_in_every_layout() {
        let hostile = names(&["x\x1b[2Jy", "ok"]);
        for args in [
            quoting_args(QuotingStyle::ShellEscape),
            Args {
                columns: true,
                width: Some(80),
                ..quoting_args(QuotingStyle::Escape)
            },
            Args {
                commas: true,
                width: Some(80),
                ..quoting_args(QuotingStyle::C)
            },
        ] {
            let out = render_with(args, &hostile);
            assert!(!out.contains('\x1b'), "{out:?}");
        }
    }

    #[test]
    fn test_columns_measure_the_quoted_width() {
        // 'a b' is 5 columns wide once quoted, so two cells need 5 + 2 + 5
        let args = Args {
            columns: true,
            width: Some(12),
            ..quoting_args(QuotingStyle::ShellAlways)
        };
        assert_eq!(render_with(args, &names(&["a b", "c d"])), "'a b'  'c d'\n");
        let narrow = Args {
            columns: true,
            width: Some(11),
            ..quoting_args(QuotingStyle::ShellAlways)
        };
        assert_eq!(
            render_with(narrow, &names(&["a b", "c d"])),
            "'a b'\n'c d'\n"
        );
    }

    #[test]
    fn test_long_format_quotes_name_and_link_target() {
        let mut link = entry(b"my link");
        link.file_type = FileType::Symlink;
        link.symlink_target = Some("tg\x1bt".into());
        let args = Args {
            color: Some(crate::cli::ColorWhen::Never),
            long: true,
            ..quoting_args(QuotingStyle::ShellEscape)
        };
        let mut buf = Vec::new();
        Formatter::new(&args)
            .format_long(&mut buf, &[link])
            .unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.trim_end().ends_with("'my link' -> 'tg'$'\\033''t'"),
            "{out}"
        );
    }

    #[test]
    fn test_heading_is_quoted() {
        let args = quoting_args(QuotingStyle::ShellEscape);
        let mut buf = Vec::new();
        Formatter::new(&args)
            .write_heading(&mut buf, std::path::Path::new("dir\x1b[1m/sub dir"))
            .unwrap();
        assert_eq!(buf, b"'dir'$'\\033''[1m/sub dir':\n");
    }

    #[test]
    fn test_extreme_timestamps_do_not_panic() {
        let now = SystemTime::now();
        let far = |secs: u64, future: bool| {
            let d = Duration::from_secs(secs);
            if future {
                SystemTime::UNIX_EPOCH.checked_add(d)
            } else {
                SystemTime::UNIX_EPOCH.checked_sub(d)
            }
        };
        let styles = [
            TimeStyle::Locale,
            TimeStyle::Iso,
            TimeStyle::LongIso,
            TimeStyle::FullIso,
            TimeStyle::Custom("%Y".to_string()),
        ];
        for secs in [10_000_000_000_000_u64, u64::MAX / 4, i64::MAX as u64] {
            for future in [true, false] {
                let Some(t) = far(secs, future) else { continue };
                for style in &styles {
                    // must not panic; an unrepresentable time prints as "?"
                    let shown = format_time(t, now, style);
                    assert!(!shown.is_empty());
                }
            }
        }
    }
}
