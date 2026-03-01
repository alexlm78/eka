use clap::{Parser, ValueHint};

/// Sort options for directory listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortField {
    /// Sort by name (default)
    #[default]
    Name,
    /// Sort by file size
    Size,
    /// Sort by modification time
    Time,
    /// Sort by access time
    Access,
    /// Sort by change time
    Change,
    /// Sort by extension
    Extension,
}

impl SortField {
    /// Parse sort field from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => None,
            "name" | "filename" => Some(SortField::Name),
            "size" | "S" => Some(SortField::Size),
            "time" | "t" => Some(SortField::Time),
            "access" | "u" => Some(SortField::Access),
            "change" | "c" => Some(SortField::Change),
            "extension" | "X" => Some(SortField::Extension),
            _ => None,
        }
    }
}

/// Command-line arguments structure for eka
#[derive(Parser, Debug)]
#[command(name = "eka")]
#[command(version = "0.2.0")]
#[command(about = "A modern replacement for ls with colors", long_about = None)]
pub struct Args {
    /// List one file per line
    #[arg(short = '1', help = "List one file per line")]
    pub one_per_line: bool,

    /// Force long listing format
    #[arg(short = 'l', help = "Use a long listing format")]
    pub long: bool,

    /// List directories only
    #[arg(
        short = 'd',
        long = "directories",
        help = "List directories only"
    )]
    pub directories: bool,

    /// Show hidden files (starting with .)
    #[arg(short = 'a', long = "all", help = "Show hidden files")]
    pub all: bool,

    /// Display file sizes in human-readable format (KB, MB, GB)
    #[arg(
        short = 'H',
        long = "human-readable",
        help = "Display sizes in human-readable format"
    )]
    pub human_readable: bool,

    /// Append file type indicators (* / = > @ |)
    #[arg(
        short = 'F',
        long = "classify",
        help = "Append file type indicators"
    )]
    pub classify: bool,

    /// List subdirectories recursively
    #[arg(
        short = 'R',
        long = "recursive",
        help = "List subdirectories recursively"
    )]
    pub recursive: bool,

    /// Sort by file size (largest first)
    #[arg(
        short = 'S',
        long = "sort_size",
        help = "Sort by file size, largest first"
    )]
    pub sort_size: bool,

    /// Sort by modification time (newest first)
    #[arg(
        short = 't',
        long = "sort_time",
        help = "Sort by modification time, newest first"
    )]
    pub sort_time: bool,

    /// Sort by access time
    #[arg(
        short = 'u',
        long = "sort_access",
        help = "Sort by access time, newest first"
    )]
    pub sort_access: bool,

    /// Reverse sort order
    #[arg(
        short = 'r',
        long = "reverse",
        help = "Reverse sort order"
    )]
    pub reverse: bool,

    /// Print inode number
    #[arg(
        short = 'i',
        long = "inode",
        help = "Print inode number"
    )]
    pub inode: bool,

    /// Sort by WORD (name, size, time, extension)
    #[arg(
        long = "sort",
        value_parser = clap::builder::PossibleValuesParser::new(["name", "size", "time", "access", "change", "extension", "none"]),
        help = "Sort by WORD instead of name"
    )]
    pub sort: Option<String>,

    /// Directory(s) to list
    #[arg(
        value_hint = ValueHint::DirPath,
        default_value = "."
    )]
    pub paths: Vec<String>,
}

impl Args {
    /// Determines the sort field based on CLI arguments
    pub fn get_sort_field(&self) -> SortField {
        // Explicit sort flags take priority
        if self.sort_size {
            return SortField::Size;
        }
        if self.sort_time {
            return SortField::Time;
        }
        if self.sort_access {
            return SortField::Access;
        }
        // Check --sort option
        if let Some(ref sort_str) = self.sort {
            return SortField::from_str(sort_str).unwrap_or_default();
        }
        // Default to name
        SortField::Name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args {
            one_per_line: false,
            long: false,
            directories: false,
            all: false,
            human_readable: false,
            classify: false,
            recursive: false,
            sort_size: false,
            sort_time: false,
            sort_access: false,
            reverse: false,
            inode: false,
            sort: None,
            paths: vec![".".to_string()],
        };
        assert!(!args.one_per_line);
        assert!(!args.long);
        assert!(!args.directories);
        assert!(!args.all);
        assert!(!args.human_readable);
        assert!(!args.classify);
        assert!(!args.recursive);
        assert!(!args.sort_size);
        assert!(!args.sort_time);
        assert!(!args.sort_access);
        assert!(!args.reverse);
        assert!(!args.inode);
        assert_eq!(args.paths, vec!["."]);
    }

    #[test]
    fn test_sort_field_from_str() {
        assert_eq!(SortField::from_str("name"), Some(SortField::Name));
        assert_eq!(SortField::from_str("size"), Some(SortField::Size));
        assert_eq!(SortField::from_str("time"), Some(SortField::Time));
        assert_eq!(SortField::from_str("extension"), Some(SortField::Extension));
        assert_eq!(SortField::from_str("invalid"), None);
    }

    #[test]
    fn test_get_sort_field_size() {
        let args = Args {
            sort_size: true,
            ..Default::default()
        };
        assert_eq!(args.get_sort_field(), SortField::Size);
    }

    #[test]
    fn test_get_sort_field_time() {
        let args = Args {
            sort_time: true,
            ..Default::default()
        };
        assert_eq!(args.get_sort_field(), SortField::Time);
    }

    #[test]
    fn test_get_sort_field_from_option() {
        let args = Args {
            sort: Some("extension".to_string()),
            ..Default::default()
        };
        assert_eq!(args.get_sort_field(), SortField::Extension);
    }
}

// Implement Default for Args for testing
impl Default for Args {
    fn default() -> Self {
        Self {
            one_per_line: false,
            long: false,
            directories: false,
            all: false,
            human_readable: false,
            classify: false,
            recursive: false,
            sort_size: false,
            sort_time: false,
            sort_access: false,
            reverse: false,
            inode: false,
            sort: None,
            paths: vec![".".to_string()],
        }
    }
}
