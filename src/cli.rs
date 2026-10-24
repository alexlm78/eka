use crate::quoting::QuotingStyle;
use chrono::format::{Item, StrftimeItems};
use clap::{ArgAction, Parser, ValueEnum, ValueHint};
use std::str::FromStr;

/// Sort keys for directory listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum SortField {
    /// Sort by name (default)
    #[default]
    Name,
    /// Directory order, no sorting
    None,
    /// Sort by file size, largest first
    Size,
    /// Sort by time, newest first
    Time,
    /// Sort by extension
    Extension,
    /// Sort by version number in names
    Version,
}

/// Which timestamp is shown (-l) and used for time sorting (-t)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum TimeField {
    #[default]
    #[value(alias = "mtime", alias = "modification")]
    Modified,
    #[value(alias = "atime", alias = "access", alias = "use")]
    Accessed,
    #[value(alias = "ctime", alias = "status")]
    Changed,
}

/// Date format used by long listings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeStyle {
    Locale,
    Iso,
    LongIso,
    FullIso,
    Custom(String),
}

impl FromStr for TimeStyle {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "locale" => Ok(Self::Locale),
            "iso" => Ok(Self::Iso),
            "long-iso" => Ok(Self::LongIso),
            "full-iso" => Ok(Self::FullIso),
            _ if value.starts_with('+') => {
                let format = &value[1..];
                // chrono panics when formatting with a specifier it does not know
                if StrftimeItems::new(format).any(|item| item == Item::Error) {
                    return Err("invalid --time-style format".to_string());
                }
                Ok(Self::Custom(format.to_string()))
            }
            _ => Err("invalid --time-style argument".to_string()),
        }
    }
}

/// Parses GNU-style block sizes in bytes.
pub fn parse_block_size(value: &str) -> Result<u64, String> {
    let (number, multiplier) = if let Some(number) = value.strip_suffix("KB") {
        (number, 1_000)
    } else if let Some(number) = value.strip_suffix("MB") {
        (number, 1_000_000)
    } else if let Some(number) = value.strip_suffix("GB") {
        (number, 1_000_000_000)
    } else if let Some(number) = value.strip_suffix("TB") {
        (number, 1_000_000_000_000)
    } else if let Some(number) = value.strip_suffix("PB") {
        (number, 1_000_000_000_000_000)
    } else if let Some(number) = value.strip_suffix("EB") {
        (number, 1_000_000_000_000_000_000)
    } else if let Some((index, suffix)) = value.char_indices().last() {
        let multiplier = match suffix {
            'K' => 1024,
            'M' => 1024_u64.pow(2),
            'G' => 1024_u64.pow(3),
            'T' => 1024_u64.pow(4),
            'P' => 1024_u64.pow(5),
            'E' => 1024_u64.pow(6),
            _ => {
                return value
                    .parse::<u64>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| "invalid --block-size argument".to_string());
            }
        };
        (&value[..index], multiplier)
    } else {
        (value, 1)
    };
    // A bare unit such as "K" or "MB" means one of that unit
    let number = if number.is_empty() && !value.is_empty() {
        "1"
    } else {
        number
    };
    number
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .and_then(|n| n.checked_mul(multiplier))
        .ok_or_else(|| "invalid --block-size argument".to_string())
}

/// When to colorize output (--color)
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorWhen {
    Always,
    Auto,
    Never,
}

/// How the short format arranges entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// One entry per line
    Single,
    /// Columns, sorted vertically (-C)
    Columns,
    /// Columns, sorted horizontally (-x)
    Across,
    /// Comma separated, wrapped to the width (-m)
    Commas,
    /// One detailed entry per line (-l)
    Long,
}

/// Command-line arguments structure for eka
#[derive(Parser, Debug, Default)]
#[command(name = "eka", version, about = "A modern replacement for ls", long_about = None)]
#[command(disable_help_flag = true)]
pub struct Args {
    /// Print help
    #[arg(long, action = ArgAction::Help)]
    pub help: Option<bool>,

    /// List one file per line
    #[arg(short = '1')]
    pub one_per_line: bool,

    /// List entries by columns
    #[arg(short = 'C')]
    pub columns: bool,

    /// List entries by lines instead of by columns
    #[arg(short = 'x')]
    pub across: bool,

    /// Fill the width with a comma separated list of entries
    #[arg(short = 'm')]
    pub commas: bool,

    /// Use a long listing format
    #[arg(short = 'l')]
    pub long: bool,

    /// Like -l, but show numeric user and group IDs
    #[arg(short = 'n', long = "numeric-uid-gid")]
    pub numeric_uid_gid: bool,

    /// Like -l, but do not list the owner
    #[arg(short = 'g')]
    pub no_owner: bool,

    /// Like -l, but do not list the group
    #[arg(short = 'o')]
    pub no_group_short: bool,

    /// In long format, do not print the group column
    #[arg(short = 'G', long = "no-group")]
    pub no_group: bool,

    /// Set output width to COLS; 0 means no limit
    #[arg(short = 'w', long = "width", value_name = "COLS")]
    pub width: Option<usize>,

    /// List directories themselves, not their contents
    #[arg(short = 'd', long = "directory")]
    pub directories: bool,

    /// Do not ignore entries whose names start with a dot
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Do not list implied . and ..
    #[arg(short = 'A', long = "almost-all")]
    pub almost_all: bool,

    /// With -l, print sizes in human readable format (1K 234M 2G)
    #[arg(short = 'h', long = "human-readable")]
    pub human_readable: bool,

    /// Print allocated size in blocks
    #[arg(short = 's', long = "size")]
    pub size: bool,

    /// Use 1024-byte blocks (the default)
    #[arg(short = 'k', long = "kibibytes")]
    pub kibibytes: bool,

    /// Scale displayed sizes by SIZE bytes
    #[arg(long, value_name = "SIZE", value_parser = parse_block_size)]
    pub block_size: Option<u64>,

    /// Append indicator (one of */=>@|) to entries
    #[arg(short = 'F', long = "classify")]
    pub classify: bool,

    /// Append / to directories
    #[arg(short = 'p', long = "indicator-style", value_name = "STYLE", value_parser = ["slash"], default_missing_value = "slash", num_args = 0..=1, require_equals = true)]
    pub indicator_style: Option<String>,

    /// List subdirectories recursively
    #[arg(short = 'R', long = "recursive")]
    pub recursive: bool,

    /// Sort by file size, largest first
    #[arg(short = 'S')]
    pub sort_size: bool,

    /// Sort by time, newest first
    #[arg(short = 't')]
    pub sort_time: bool,

    /// Sort by version number in names
    #[arg(short = 'v')]
    pub sort_version: bool,

    /// Sort by extension
    #[arg(short = 'X')]
    pub sort_extension: bool,

    /// Do not sort; list entries in directory order
    #[arg(short = 'U')]
    pub sort_none: bool,

    /// With -lt: sort by, and show, access time; with -l: show access time
    /// and sort by name; otherwise: sort by access time, newest first
    #[arg(short = 'u')]
    pub sort_access: bool,

    /// With -lt: sort by, and show, ctime; with -l: show ctime and sort by
    /// name; otherwise: sort by ctime, newest first
    #[arg(short = 'c')]
    pub sort_change: bool,

    /// Select the timestamp shown by -l and used by -t
    #[arg(long = "time", value_enum, value_name = "WORD")]
    pub time: Option<TimeField>,

    /// Select the timestamp format used by -l
    #[arg(long = "time-style", value_name = "STYLE")]
    pub time_style: Option<TimeStyle>,

    /// Like -l --time-style=full-iso
    #[arg(long = "full-time")]
    pub full_time: bool,

    /// Reverse order while sorting
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

    /// List directories before other entries
    #[arg(long = "group-directories-first")]
    pub group_directories_first: bool,

    /// Do not list entries matching PATTERN
    #[arg(short = 'I', long = "ignore", value_name = "PATTERN", action = ArgAction::Append)]
    pub ignore: Vec<String>,

    /// Do not list entries matching PATTERN unless -a or -A is given
    #[arg(long = "hide", value_name = "PATTERN", action = ArgAction::Append)]
    pub hide: Vec<String>,

    /// Do not list entries ending in ~
    #[arg(short = 'B', long = "ignore-backups")]
    pub ignore_backups: bool,

    /// Print entry names without quoting
    #[arg(short = 'N', long = "literal")]
    pub literal: bool,

    /// Print C-style escapes for nongraphic characters
    #[arg(short = 'b', long = "escape")]
    pub escape: bool,

    /// Enclose entry names in double quotes
    #[arg(short = 'Q', long = "quote-name")]
    pub quote_name: bool,

    /// Print ? instead of nongraphic characters
    #[arg(short = 'q', long = "hide-control-chars")]
    pub hide_control_chars: bool,

    /// Show nongraphic characters as is (the default when not a terminal)
    #[arg(long = "show-control-chars")]
    pub show_control_chars: bool,

    /// Use quoting style WORD for entry names
    #[arg(long = "quoting-style", value_enum, value_name = "WORD")]
    pub quoting_style: Option<QuotingStyle>,

    /// Print the index number of each file
    #[arg(short = 'i', long = "inode")]
    pub inode: bool,

    /// Follow symbolic links listed on the command line
    #[arg(short = 'H', long = "dereference-command-line")]
    pub deref_cmdline: bool,

    /// When showing file information for a symbolic link, show the
    /// information for the file the link references
    #[arg(short = 'L', long = "dereference")]
    pub deref_all: bool,

    /// Do not follow symbolic links (default; overrides -H and -L)
    #[arg(short = 'P')]
    pub no_deref: bool,

    /// Sort by WORD instead of name
    #[arg(long, value_enum, value_name = "WORD")]
    pub sort: Option<SortField>,

    /// Colorize the output
    #[arg(
        long,
        value_enum,
        value_name = "WHEN",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "always"
    )]
    pub color: Option<ColorWhen>,

    /// Files or directories to list
    #[arg(value_hint = ValueHint::AnyPath, default_value = ".")]
    pub paths: Vec<String>,
}

impl Args {
    /// Whether any option requires the long listing format.
    pub fn is_long(&self) -> bool {
        self.long || self.numeric_uid_gid || self.no_owner || self.no_group_short || self.full_time
    }

    /// Sort key precedence: --sort=WORD, -U, -S, -t, -v, -X, then name.
    /// With -l, -u/-c only change the displayed time.
    pub fn sort_field(&self) -> SortField {
        if let Some(s) = self.sort {
            s
        } else if self.sort_none {
            SortField::None
        } else if self.sort_size {
            SortField::Size
        } else if self.sort_time || ((self.sort_access || self.sort_change) && !self.is_long()) {
            SortField::Time
        } else if self.sort_version {
            SortField::Version
        } else if self.sort_extension {
            SortField::Extension
        } else {
            SortField::Name
        }
    }

    /// Timestamp used for display and time sorting
    pub fn time_field(&self) -> TimeField {
        if let Some(time) = self.time {
            time
        } else if self.sort_change {
            TimeField::Changed
        } else if self.sort_access {
            TimeField::Accessed
        } else {
            TimeField::Modified
        }
    }

    /// Output arrangement. Precedence: -l, -m, -x, -C, -1; with none of them,
    /// columns on a terminal and one entry per line otherwise (like ls).
    pub fn layout(&self, stdout_is_tty: bool) -> Layout {
        if self.is_long() {
            Layout::Long
        } else if self.commas {
            Layout::Commas
        } else if self.across {
            Layout::Across
        } else if self.columns {
            Layout::Columns
        } else if self.one_per_line || !stdout_is_tty {
            Layout::Single
        } else {
            Layout::Columns
        }
    }

    /// Line width for column layouts: -w, then $COLUMNS, then the terminal,
    /// then 80. Zero means unlimited.
    pub fn line_width(&self) -> usize {
        if let Some(w) = self.width {
            return w;
        }
        if let Some(w) = std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            && w > 0
        {
            return w;
        }
        terminal_size::terminal_size().map_or(80, |(w, _)| w.0 as usize)
    }

    /// Quoting style. Precedence: --quoting-style, -Q, -b, -N, $QUOTING_STYLE;
    /// the default is `shell-escape` on a terminal and `literal` otherwise.
    pub fn quoting_style(&self, stdout_is_tty: bool) -> QuotingStyle {
        select_quoting_style(
            self,
            stdout_is_tty,
            std::env::var("QUOTING_STYLE").ok().as_deref(),
        )
    }

    /// Whether unprintable characters print as `?` in the styles that show
    /// them: -q, or by default on a terminal; --show-control-chars turns it off.
    pub fn hide_control(&self, stdout_is_tty: bool) -> bool {
        if self.show_control_chars {
            false
        } else {
            self.hide_control_chars || stdout_is_tty
        }
    }

    /// True when hidden entries are listed (-a or -A)
    pub fn show_hidden(&self) -> bool {
        self.all || self.almost_all
    }

    /// Follow every symlink (-L), unless -P is given
    pub fn dereference(&self) -> bool {
        self.deref_all && !self.no_deref
    }

    /// Follow symlinks named on the command line (-H or -L), unless -P is given
    pub fn dereference_cmdline(&self) -> bool {
        (self.deref_cmdline || self.deref_all) && !self.no_deref
    }

    /// Effective time style, with TIME_STYLE as a fallback.
    pub fn time_style(&self) -> TimeStyle {
        select_time_style(
            self.time_style.as_ref(),
            self.full_time,
            std::env::var("TIME_STYLE").ok().as_deref(),
        )
    }

    /// Whether -p requests directory slash indicators.
    pub fn slash_indicator(&self) -> bool {
        self.indicator_style.as_deref() == Some("slash")
    }
}

fn select_quoting_style(args: &Args, tty: bool, environment: Option<&str>) -> QuotingStyle {
    if let Some(style) = args.quoting_style {
        style
    } else if args.quote_name {
        QuotingStyle::C
    } else if args.escape {
        QuotingStyle::Escape
    } else if args.literal {
        QuotingStyle::Literal
    } else if let Some(style) = environment.and_then(|v| QuotingStyle::from_str(v, true).ok()) {
        style
    } else if tty {
        QuotingStyle::ShellEscape
    } else {
        QuotingStyle::Literal
    }
}

fn select_time_style(
    explicit: Option<&TimeStyle>,
    full_time: bool,
    environment: Option<&str>,
) -> TimeStyle {
    if full_time {
        TimeStyle::FullIso
    } else if let Some(style) = explicit {
        style.clone()
    } else {
        environment
            .and_then(|style| style.parse().ok())
            .unwrap_or(TimeStyle::Locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::try_parse_from(std::iter::once("eka").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn test_default_sort_is_name() {
        assert_eq!(parse(&[]).sort_field(), SortField::Name);
        assert_eq!(parse(&[]).paths, vec!["."]);
    }

    #[test]
    fn test_size_and_time_sort() {
        assert_eq!(parse(&["-S"]).sort_field(), SortField::Size);
        assert_eq!(parse(&["-t"]).sort_field(), SortField::Time);
    }

    #[test]
    fn test_sort_word_overrides_flags() {
        assert_eq!(
            parse(&["-S", "--sort=extension"]).sort_field(),
            SortField::Extension
        );
        assert_eq!(parse(&["--sort=none"]).sort_field(), SortField::None);
    }

    #[test]
    fn test_sort_short_options_and_precedence() {
        assert_eq!(parse(&["-v"]).sort_field(), SortField::Version);
        assert_eq!(parse(&["-X"]).sort_field(), SortField::Extension);
        assert_eq!(parse(&["-U"]).sort_field(), SortField::None);
        assert_eq!(parse(&["-vX"]).sort_field(), SortField::Version);
        assert_eq!(parse(&["-tv"]).sort_field(), SortField::Time);
        assert_eq!(parse(&["-StvX"]).sort_field(), SortField::Size);
        assert_eq!(parse(&["-UStvX"]).sort_field(), SortField::None);
        assert_eq!(
            parse(&["--sort=version", "-U"]).sort_field(),
            SortField::Version
        );
    }

    #[test]
    fn test_grouping_and_filter_options() {
        let args = parse(&[
            "--group-directories-first",
            "-I",
            "*.tmp",
            "--ignore=*.bak",
            "--hide=*.o",
            "-B",
        ]);
        assert!(args.group_directories_first && args.ignore_backups);
        assert_eq!(args.ignore, ["*.tmp", "*.bak"]);
        assert_eq!(args.hide, ["*.o"]);
    }

    #[test]
    fn test_access_time_semantics() {
        // -u alone sorts by atime; with -l it only changes the displayed time
        assert_eq!(parse(&["-u"]).sort_field(), SortField::Time);
        assert_eq!(parse(&["-u"]).time_field(), TimeField::Accessed);
        assert_eq!(parse(&["-lu"]).sort_field(), SortField::Name);
        assert_eq!(parse(&["-lut"]).sort_field(), SortField::Time);
        assert_eq!(parse(&["-c"]).time_field(), TimeField::Changed);
    }

    #[test]
    fn test_h_is_human_readable_not_help() {
        let a = parse(&["-lh"]);
        assert!(a.long && a.human_readable);
    }

    #[test]
    fn test_almost_all_vs_all() {
        let a = parse(&["-A"]);
        assert!(a.show_hidden() && !a.all);
        assert!(parse(&["-a"]).all);
    }

    #[test]
    fn test_dereference_flags() {
        assert!(!parse(&[]).dereference_cmdline());
        assert!(parse(&["-H"]).dereference_cmdline() && !parse(&["-H"]).dereference());
        assert!(parse(&["-L"]).dereference() && parse(&["-L"]).dereference_cmdline());
        assert!(!parse(&["-LP"]).dereference());
    }

    #[test]
    fn test_color_flag_forms() {
        assert_eq!(parse(&["--color"]).color, Some(ColorWhen::Always));
        assert_eq!(parse(&["--color=never"]).color, Some(ColorWhen::Never));
        assert_eq!(parse(&["--color=auto"]).color, Some(ColorWhen::Auto));
        assert_eq!(parse(&[]).color, None);
    }

    #[test]
    fn test_layout_precedence() {
        assert_eq!(parse(&[]).layout(false), Layout::Single);
        assert_eq!(parse(&[]).layout(true), Layout::Columns);
        assert_eq!(parse(&["-1"]).layout(true), Layout::Single);
        assert_eq!(parse(&["-C"]).layout(false), Layout::Columns);
        assert_eq!(parse(&["-x"]).layout(false), Layout::Across);
        assert_eq!(parse(&["-m"]).layout(true), Layout::Commas);
        assert_eq!(parse(&["-l", "-m"]).layout(true), Layout::Long);
    }

    #[test]
    fn test_width_option() {
        assert_eq!(parse(&["-w", "40"]).line_width(), 40);
        assert_eq!(parse(&["--width=0"]).line_width(), 0);
    }

    #[test]
    fn test_quoting_style_defaults_and_precedence() {
        let style = |a: &[&str], tty, env| select_quoting_style(&parse(a), tty, env);
        assert_eq!(style(&[], false, None), QuotingStyle::Literal);
        assert_eq!(style(&[], true, None), QuotingStyle::ShellEscape);
        assert_eq!(style(&["-N"], true, None), QuotingStyle::Literal);
        assert_eq!(style(&["-b"], false, None), QuotingStyle::Escape);
        assert_eq!(style(&["-Q"], false, None), QuotingStyle::C);
        assert_eq!(style(&["-Q", "-b"], false, None), QuotingStyle::C);
        assert_eq!(
            style(&["-Q", "--quoting-style=shell"], false, None),
            QuotingStyle::Shell
        );
        // environment sits below the options and above the default
        assert_eq!(style(&[], true, Some("c")), QuotingStyle::C);
        assert_eq!(style(&["-b"], true, Some("c")), QuotingStyle::Escape);
        assert_eq!(style(&[], true, Some("bogus")), QuotingStyle::ShellEscape);
        assert!(Args::try_parse_from(["eka", "--quoting-style=nope"]).is_err());
    }

    #[test]
    fn test_hide_control_defaults() {
        assert!(!parse(&[]).hide_control(false));
        assert!(parse(&[]).hide_control(true));
        assert!(parse(&["-q"]).hide_control(false));
        assert!(!parse(&["--show-control-chars"]).hide_control(true));
        assert!(!parse(&["-q", "--show-control-chars"]).hide_control(true));
    }

    #[test]
    fn test_gnu_long_options() {
        let a = parse(&[
            "--all",
            "--reverse",
            "--recursive",
            "--directory",
            "--inode",
        ]);
        assert!(a.all && a.reverse && a.recursive && a.directories && a.inode);
    }

    #[test]
    fn test_implied_long_options_and_no_group() {
        for option in ["-n", "--numeric-uid-gid", "-g", "-o", "--full-time"] {
            assert!(parse(&[option]).is_long(), "{option}");
        }
        assert!(!parse(&["-G"]).is_long());
        assert_eq!(parse(&["-n", "-u"]).sort_field(), SortField::Name);
    }

    #[test]
    fn test_new_formatting_options() {
        let a = parse(&["-s", "-k", "--block-size=2M", "-p"]);
        assert!(a.size && a.kibibytes && a.slash_indicator());
        assert_eq!(a.block_size, Some(2 * 1024 * 1024));
        assert!(parse(&["-F", "-p"]).classify);
        assert!(parse(&["--indicator-style=slash"]).slash_indicator());
    }

    #[test]
    fn test_block_size_parser() {
        assert_eq!(parse_block_size("1").unwrap(), 1);
        assert_eq!(parse_block_size("2K").unwrap(), 2048);
        assert_eq!(parse_block_size("3M").unwrap(), 3 * 1024 * 1024);
        assert_eq!(parse_block_size("4KB").unwrap(), 4000);
        assert_eq!(parse_block_size("5GB").unwrap(), 5_000_000_000);
        // a bare unit means one of it
        assert_eq!(parse_block_size("K").unwrap(), 1024);
        assert_eq!(parse_block_size("M").unwrap(), 1024 * 1024);
        assert_eq!(parse_block_size("KB").unwrap(), 1000);
        assert!(parse_block_size("").is_err());
        assert!(parse_block_size("0").is_err());
        assert!(parse_block_size("0K").is_err());
        assert!(parse_block_size("12Q").is_err());
        assert!(Args::try_parse_from(["eka", "--block-size=wat"]).is_err());
    }

    #[test]
    fn test_time_option_aliases_and_styles() {
        assert_eq!(parse(&["--time=access"]).time_field(), TimeField::Accessed);
        assert_eq!(parse(&["--time=status"]).time_field(), TimeField::Changed);
        assert_eq!(parse(&["--time=mtime"]).time_field(), TimeField::Modified);
        assert_eq!(
            parse(&["--time=access", "-t"]).sort_field(),
            SortField::Time
        );
        assert_eq!("+%F".parse(), Ok(TimeStyle::Custom("%F".to_string())));
        assert!("bad".parse::<TimeStyle>().is_err());
        // formats chrono cannot render are rejected up front, not at print time
        assert!("+%Q".parse::<TimeStyle>().is_err());
        assert!("+%".parse::<TimeStyle>().is_err());
        assert!("+%Y-%m-%d %H:%M:%S".parse::<TimeStyle>().is_ok());
        assert!("+recent %H:%M\nold %Y".parse::<TimeStyle>().is_ok());
        assert_eq!(
            select_time_style(Some(&TimeStyle::Iso), false, Some("full-iso")),
            TimeStyle::Iso
        );
        assert_eq!(
            select_time_style(None, false, Some("long-iso")),
            TimeStyle::LongIso
        );
        assert_eq!(
            select_time_style(None, false, Some("bad")),
            TimeStyle::Locale
        );
    }
}
