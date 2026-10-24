use crate::types::{FileEntry, FileType};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;

/// ANSI color codes for different file types
pub struct Colors;

impl Colors {
    /// Retrieves the ANSI color code for a given file type
    pub fn get_color(file_type: FileType) -> &'static str {
        match file_type {
            FileType::Directory => "\x1b[36m",    // Cyan
            FileType::Executable => "\x1b[32m",   // Bright green
            FileType::Symlink => "\x1b[33m",      // Yellow
            FileType::BrokenSymlink => "\x1b[31;1m", // Bold red
            FileType::Fifo => "\x1b[33m",         // Yellow
            FileType::Socket => "\x1b[35m",       // Magenta
            FileType::CharDevice | FileType::BlockDevice => "\x1b[33;1m", // Bold yellow
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

    /// Writes the filename (raw bytes), colored, with optional classification indicator
    pub fn write_filename(out: &mut impl Write, entry: &FileEntry, classify: bool) -> io::Result<()> {
        let color = if entry.is_hidden && !entry.is_symlink {
            Self::get_hidden_color()
        } else {
            Self::get_color(entry.file_type)
        };
        out.write_all(color.as_bytes())?;
        out.write_all(entry.name.as_bytes())?;
        out.write_all(Self::reset().as_bytes())?;

        if let Some(ref target) = entry.symlink_target {
            out.write_all(b" \x1b[90m-> ")?;
            out.write_all(target.as_bytes())?;
            out.write_all(Self::reset().as_bytes())?;
        }

        if classify {
            let indicator = entry.file_type.indicator();
            if !indicator.is_empty() {
                write!(out, "{}{}{}", Self::get_color(entry.file_type), indicator, Self::reset())?;
            }
        }
        Ok(())
    }
}

/// Helper struct for colored output in long format
pub struct ColoredOutput;

impl ColoredOutput {
    /// Prints permissions with colors (directories in cyan)
    pub fn write_permissions(out: &mut impl Write, permissions: &str, is_dir: bool) -> io::Result<()> {
        if is_dir {
            write!(out, "\x1b[36m{}\x1b[0m", permissions)
        } else {
            write!(out, "{}", permissions)
        }
    }

    /// Prints size in human-readable format
    pub fn write_human_size(out: &mut impl Write, size: u64) -> io::Result<()> {
        let (value, unit) = Self::human_readable_size(size);
        write!(out, "{:>5}{}", value, unit)
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
