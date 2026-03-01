use crate::types::FileType;

/// ANSI color codes for different file types
pub struct Colors;

impl Colors {
    /// Retrieves the ANSI color code for a given file type
    pub fn get_color(file_type: FileType) -> &'static str {
        match file_type {
            FileType::Directory => "\x1b[36m",    // Cyan
            FileType::Executable => "\x1b[32m",   // Bright green
            FileType::Symlink => "\x1b[33m",      // Yellow
            FileType::Image => "\x1b[35m",        // Magenta
            FileType::Video => "\x1b[31m",        // Red
            FileType::Audio => "\x1b[38;5;208m", // Orange
            FileType::Compressed => "\x1b[34m",   // Purple/Blue
            FileType::Regular => "\x1b[37m",      // White
        }
    }

    /// Retrieves the color code for hidden files
    pub fn get_hidden_color() -> &'static str {
        "\x1b[90m" // Dark gray
    }

    /// Returns the reset code to restore default terminal colors
    pub fn reset() -> &'static str {
        "\x1b[0m"
    }

    /// Prints text with the appropriate color based on file type
    pub fn print_with_color(text: &str, file_type: FileType, is_hidden: bool) {
        let color = if is_hidden {
            Self::get_hidden_color()
        } else {
            Self::get_color(file_type)
        };
        print!("{}{}{}", color, text, Self::reset());
    }

    /// Prints a complete line with colors
    #[allow(dead_code)]
    pub fn print_colored_line(text: &str, file_type: FileType, is_hidden: bool) {
        Self::print_with_color(text, file_type, is_hidden);
        println!();
    }

    /// Prints the filename with its characteristic color
    pub fn print_filename(entry: &crate::types::FileEntry) {
        // For symlinks, display the target in a different color
        if entry.is_symlink {
            let color = Self::get_color(entry.file_type);
            if let Some(ref target) = entry.symlink_target {
                print!("{}{} ", color, entry.name);
                print!("\x1b[90m→ {}{}", target, Self::reset());
            } else {
                print!("{}{}{}", color, entry.name, Self::reset());
            }
        } else {
            Self::print_with_color(&entry.name, entry.file_type, entry.is_hidden);
        }
    }
}

/// Helper struct for colored output in long format
pub struct ColoredOutput;

impl ColoredOutput {
    /// Prints permissions with colors (directories in cyan)
    pub fn print_permissions(permissions: &str, is_dir: bool) {
        if is_dir {
            print!("\x1b[36m{}\x1b[0m", permissions);
        } else {
            print!("{}", permissions);
        }
    }

    /// Prints size with color coding (small in green, large in yellow)
    #[allow(dead_code)]
    pub fn print_size(size: u64) {
        if size < 1024 {
            print!("\x1b[32m{:>4}\x1b[0m ", size);
        } else if size < 1024 * 1024 {
            print!("\x1b[33m{:>3}K\x1b[0m ", size / 1024);
        } else if size < 1024 * 1024 * 1024 {
            print!("\x1b[33m{:>3}M\x1b[0m ", size / (1024 * 1024));
        } else {
            print!("\x1b[33m{:>3}G\x1b[0m ", size / (1024 * 1024 * 1024));
        }
    }

    /// Prints size in human-readable format
    pub fn print_human_size(size: u64) {
        let (value, unit) = Self::human_readable_size(size);
        print!("{:>5}{}", value, unit);
    }

    fn human_readable_size(size: u64) -> (String, String) {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if size >= GB {
            let gb = size as f64 / GB as f64;
            (format!("{:>4.1}", gb), "G".to_string())
        } else if size >= MB {
            let mb = size as f64 / MB as f64;
            (format!("{:>4.1}", mb), "M".to_string())
        } else if size >= KB {
            let kb = size as f64 / KB as f64;
            (format!("{:>4.1}", kb), "K".to_string())
        } else {
            (format!("{:>4}", size), "B".to_string())
        }
    }
}
