use clap::{ArgAction, Parser, ValueEnum, ValueHint};

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
}

/// Which timestamp is shown (-l) and used for time sorting (-t)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeField {
    #[default]
    Modified,
    Accessed,
    Changed,
}

/// When to colorize output (--color)
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorWhen {
    Always,
    Auto,
    Never,
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

    /// Use a long listing format
    #[arg(short = 'l')]
    pub long: bool,

    /// List directories themselves, not their contents
    #[arg(short = 'd', long = "directory")]
    pub directories: bool,

    /// Do not ignore entries starting with .
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Do not list implied . and ..
    #[arg(short = 'A', long = "almost-all")]
    pub almost_all: bool,

    /// With -l, print sizes in human readable format (1K 234M 2G)
    #[arg(short = 'h', long = "human-readable")]
    pub human_readable: bool,

    /// Append indicator (one of */=>@|) to entries
    #[arg(short = 'F', long = "classify")]
    pub classify: bool,

    /// List subdirectories recursively
    #[arg(short = 'R', long = "recursive")]
    pub recursive: bool,

    /// Sort by file size, largest first
    #[arg(short = 'S')]
    pub sort_size: bool,

    /// Sort by time, newest first
    #[arg(short = 't')]
    pub sort_time: bool,

    /// With -lt: sort by, and show, access time; with -l: show access time
    /// and sort by name; otherwise: sort by access time, newest first
    #[arg(short = 'u')]
    pub sort_access: bool,

    /// With -lt: sort by, and show, ctime; with -l: show ctime and sort by
    /// name; otherwise: sort by ctime, newest first
    #[arg(short = 'c')]
    pub sort_change: bool,

    /// Reverse order while sorting
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

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
    /// Sort key. `--sort` wins; with -l, -u/-c only change the displayed time.
    pub fn sort_field(&self) -> SortField {
        if let Some(s) = self.sort {
            s
        } else if self.sort_size {
            SortField::Size
        } else if self.sort_time || ((self.sort_access || self.sort_change) && !self.long) {
            SortField::Time
        } else {
            SortField::Name
        }
    }

    /// Timestamp used for display and time sorting
    pub fn time_field(&self) -> TimeField {
        if self.sort_change {
            TimeField::Changed
        } else if self.sort_access {
            TimeField::Accessed
        } else {
            TimeField::Modified
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
        assert_eq!(parse(&["-S", "--sort=extension"]).sort_field(), SortField::Extension);
        assert_eq!(parse(&["--sort=none"]).sort_field(), SortField::None);
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
    fn test_gnu_long_options() {
        let a = parse(&["--all", "--reverse", "--recursive", "--directory", "--inode"]);
        assert!(a.all && a.reverse && a.recursive && a.directories && a.inode);
    }
}
