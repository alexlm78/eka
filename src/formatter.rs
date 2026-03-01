use crate::colors::{Colors, ColoredOutput};
use crate::types::{FileEntry, FileType};
use chrono::{DateTime, Local};

/// Output formatter for eka
pub struct Formatter;

impl Formatter {
    /// Prints entries in short format (vertical list)
    pub fn format_short(entries: &[FileEntry]) {
        for entry in entries {
            Colors::print_filename(entry);
            println!();
        }
    }

    /// Prints entries in long format (with columns)
    pub fn format_long(entries: &[FileEntry], human_readable: bool) {
        // Column headers
        println!(
            "{:<18} {:>4} {:<8} {:<8} {:>10} {:>20} {}",
            "Permissions",
            "Links",
            "User",
            "Group",
            "Size",
            "Date",
            "Name"
        );

        for entry in entries {
            Self::print_long_entry(entry, human_readable);
        }
    }

    /// Prints a single entry in long format
    fn print_long_entry(entry: &FileEntry, human_readable: bool) {
        let is_dir = entry.file_type == FileType::Directory;

        // Colored permissions
        print!("");
        ColoredOutput::print_permissions(&entry.permissions, is_dir);
        print!("  ");

        // Number of hard links (always 1 for simplicity)
        print!("{:>4} ", 1);

        // User and group (simplified)
        print!("{:<8} ", "user");
        print!("{:<8} ", "staff");

        // Size
        if human_readable {
            ColoredOutput::print_human_size(entry.size);
            print!(" ");
        } else {
            print!("{:>10} ", entry.size);
        }

        // Modification date
        let datetime: DateTime<Local> = entry.modified.into();
        let formatted = datetime.format("%b %d %H:%M").to_string();
        print!("{:>20} ", formatted);

        // Colored name
        Colors::print_filename(entry);

        println!();
    }

    /// Prints the directory header
    pub fn print_header(path: &str, total_entries: usize) {
        if total_entries > 0 {
            println!("{}:", path);
            println!("total {}", total_entries);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_creation() {
        let formatter = Formatter;
        // Basic test to ensure formatter can be created
        assert!(true);
    }
}
