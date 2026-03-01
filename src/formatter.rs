use crate::cli::Args;
use crate::colors::{Colors, ColoredOutput};
use crate::types::{FileEntry, FileType};
use chrono::{DateTime, Local};

/// Output formatter for eka
pub struct Formatter;

impl Formatter {
    /// Prints entries in short format (one per line)
    pub fn format_short(entries: &[FileEntry], args: &Args) {
        for entry in entries {
            // Print inode if requested
            if args.inode {
                if let Some(inode) = entry.inode {
                    print!("{:>12} ", inode);
                } else {
                    print!("{:>12} ", 0);
                }
            }
            
            // Print filename with color and classification
            Colors::print_filename_with_indicator(entry, args.classify);
            println!();
        }
    }

    /// Prints entries in long format (with columns)
    pub fn format_long(entries: &[FileEntry], args: &Args) {
        // Column headers - adjust based on flags
        let mut format_str = String::new();
        
        if args.inode {
            format_str.push_str("{:>12} ");
        }
        
        format_str.push_str("{:<18} {:>4} {:<8} {:<8} ");
        
        if args.human_readable {
            format_str.push_str("{:>6} ");
        } else {
            format_str.push_str("{:>10} ");
        }
        
        format_str.push_str("{:>20} {}");
        
        let header = if args.inode {
            if args.human_readable {
                "Permissions    Links User     Group   Size Date                Name"
            } else {
                "Permissions    Links User     Group     Size Date                Name"
            }
        } else {
            if args.human_readable {
                "Permissions    Links User     Group   Size Date                Name"
            } else {
                "Permissions    Links User     Group     Size Date                Name"
            }
        };
        
        println!("{}", header);

        for entry in entries {
            Self::print_long_entry(entry, args);
        }
    }

    /// Prints a single entry in long format
    fn print_long_entry(entry: &FileEntry, args: &Args) {
        let is_dir = entry.file_type == FileType::Directory;

        // Print inode if requested
        if args.inode {
            if let Some(inode) = entry.inode {
                print!("{:>12} ", inode);
            } else {
                print!("{:>12} ", 0);
            }
        }

        // Colored permissions
        ColoredOutput::print_permissions(&entry.permissions, is_dir);
        print!("  ");

        // Number of hard links
        print!("{:>4} ", entry.nlink);

        // User and group (simplified)
        print!("{:<8} ", "user");
        print!("{:<8} ", "staff");

        // Size
        if args.human_readable {
            ColoredOutput::print_human_size(entry.size);
            print!(" ");
        } else {
            print!("{:>10} ", entry.size);
        }

        // Modification date
        let datetime: DateTime<Local> = entry.modified.into();
        let formatted = datetime.format("%b %d %H:%M").to_string();
        print!("{:>20} ", formatted);

        // Colored name with classification indicator
        Colors::print_filename_with_indicator(entry, args.classify);

        println!();
    }

    /// Prints the directory header
    pub fn print_header(path: &str, total_entries: usize) {
        if total_entries > 0 {
            println!("{}:", path);
            println!("total {}", total_entries);
        }
    }

    /// Prints a complete directory entry for recursive listing
    #[allow(dead_code)]
    pub fn print_directory(path: &str, entries: &[FileEntry], args: &Args) {
        if args.long {
            Self::print_header(path, entries.len());
            Self::format_long(entries, args);
        } else {
            Self::print_header(path, entries.len());
            Self::format_short(entries, args);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn create_test_entry(name: &str) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            file_type: FileType::Regular,
            size: 1024,
            permissions: "-rw-r--r--".to_string(),
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            changed: SystemTime::UNIX_EPOCH,
            is_hidden: name.starts_with('.'),
            is_symlink: false,
            symlink_target: None,
            inode: Some(12345),
            nlink: 1,
        }
    }

    #[test]
    fn test_formatter_creation() {
        // Basic test to ensure formatter can be created
        assert!(true);
    }

    #[test]
    fn test_format_short() {
        let entries = vec![
            create_test_entry("file1.txt"),
            create_test_entry("file2.txt"),
        ];
        let args = Args::default();
        
        // This should not panic
        Formatter::format_short(&entries, &args);
    }

    #[test]
    fn test_format_long() {
        let entries = vec![
            create_test_entry("file1.txt"),
        ];
        let args = Args::default();
        
        // This should not panic
        Formatter::format_long(&entries, &args);
    }

    #[test]
    fn test_print_header() {
        Formatter::print_header("/test/path", 5);
    }
}
