use clap::{Parser, ValueHint};

/// Command-line arguments structure for eka
#[derive(Parser, Debug)]
#[command(name = "eka")]
#[command(version = "0.1.0")]
#[command(about = "A modern replacement for ls with colors", long_about = None)]
pub struct Args {
    /// Enable long listing format with detailed information
    #[arg(short = 'l', long = "long", help = "Use long listing format")]
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

    /// Directory(s) to list
    #[arg(
        value_hint = ValueHint::DirPath,
        default_value = "."
    )]
    pub paths: Vec<String>,
}

impl Args {
    /// Creates default argument values
    pub fn default() -> Self {
        Self {
            long: false,
            directories: false,
            all: false,
            human_readable: false,
            paths: vec![".".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args::default();
        assert!(!args.long);
        assert!(!args.directories);
        assert!(!args.all);
        assert!(!args.human_readable);
        assert_eq!(args.paths, vec!["."]);
    }
}
