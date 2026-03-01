use crate::cli::{Args, SortField};
use crate::types::FileEntry;
use std::fs;
use std::path::Path;

/// Recursive directory entry with path information
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

/// Reads a directory and returns filtered entries based on CLI arguments
pub fn read_directory(path: &str, args: &Args) -> Result<Vec<FileEntry>, String> {
    let path = Path::new(path);

    // Verify path exists
    if !path.exists() {
        return Err(format!("No such directory: {}", path.display()));
    }

    // Verify it's actually a directory
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }

    // Read directory entries
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => return Err(format!("Error reading {}: {}", path.display(), e)),
    };

    // Convert to FileEntry and apply filters
    let mut file_entries: Vec<FileEntry> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            FileEntry::from_path(entry.path())
        })
        .filter(|entry| {
            // Apply filter arguments
            if args.directories && entry.file_type != crate::types::FileType::Directory {
                return false;
            }
            if !args.all && entry.is_hidden {
                return false;
            }
            true
        })
        .collect();

    // Sort entries based on CLI arguments
    sort_entries(&mut file_entries, &args);

    Ok(file_entries)
}

/// Reads directory recursively and returns all entries
pub fn read_directory_recursive(path: &str, args: &Args) -> Result<Vec<DirectoryEntry>, String> {
    let mut result: Vec<DirectoryEntry> = Vec::new();
    
    // Read the initial directory
    let entries = read_directory(path, args)?;
    result.push(DirectoryEntry {
        path: path.to_string(),
        entries,
    });

    // Recursively process subdirectories
    if args.recursive {
        // Collect subdirectory paths first to avoid borrow issues
        let mut subdirs: Vec<String> = Vec::new();
        
        for entry in &result[0].entries {
            if entry.file_type == crate::types::FileType::Directory 
                && !entry.is_hidden 
                && entry.name != "." 
                && entry.name != ".." 
            {
                subdirs.push(format!("{}/{}", path, entry.name));
            }
        }
        
        // Process subdirectories
        for subdir in subdirs {
            if let Ok(subdir_entries) = read_directory_recursive(&subdir, args) {
                result.extend(subdir_entries);
            }
        }
    }

    Ok(result)
}

/// Sorts entries based on CLI arguments
fn sort_entries(entries: &mut Vec<FileEntry>, args: &Args) {
    let sort_field = args.get_sort_field();
    let reverse = args.reverse;
    
    entries.sort_by(|a, b| {
        // First compare by type (directories first), but only for name sorting
        if sort_field == SortField::Name {
            let a_is_dir = a.file_type == crate::types::FileType::Directory;
            let b_is_dir = b.file_type == crate::types::FileType::Directory;
            
            if a_is_dir != b_is_dir {
                let cmp = b_is_dir.cmp(&a_is_dir);
                return if reverse { cmp.reverse() } else { cmp };
            }
        }

        // Sort by the selected field
        let cmp = match sort_field {
            SortField::Name => {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
            SortField::Size => {
                // Larger files first (descending)
                b.size.cmp(&a.size)
            }
            SortField::Time | SortField::Access | SortField::Change => {
                // Newer first (descending)
                b.get_sort_time(sort_field).cmp(&a.get_sort_time(sort_field))
            }
            SortField::Extension => {
                // Get extensions
                let ext_a = Path::new(&a.name)
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                let ext_b = Path::new(&b.name)
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                ext_a.cmp(&ext_b)
            }
        };

        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

/// Extracts the base name of the path for display purposes
#[allow(dead_code)]
pub fn get_dir_name(path: &str) -> String {
    let path = Path::new(path);
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn create_test_entry(name: &str, file_type: FileType, size: u64) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            file_type,
            size,
            permissions: "-rw-r--r--".to_string(),
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            changed: SystemTime::UNIX_EPOCH,
            is_hidden: name.starts_with('.'),
            is_symlink: false,
            symlink_target: None,
            inode: None,
            nlink: 1,
        }
    }

    #[test]
    fn test_get_dir_name() {
        assert_eq!(get_dir_name("/home/user/documents"), "documents");
        assert_eq!(get_dir_name("."), ".");
    }

    #[test]
    fn test_sort_by_name() {
        let mut entries = vec![
            create_test_entry("zebra", FileType::Regular, 100),
            create_test_entry("apple", FileType::Regular, 200),
            create_test_entry("banana", FileType::Regular, 50),
        ];
        
        let args = Args::default();
        sort_entries(&mut entries, &args);
        
        assert_eq!(entries[0].name, "apple");
        assert_eq!(entries[1].name, "banana");
        assert_eq!(entries[2].name, "zebra");
    }

    #[test]
    fn test_sort_by_size() {
        let mut entries = vec![
            create_test_entry("small", FileType::Regular, 50),
            create_test_entry("large", FileType::Regular, 1000),
            create_test_entry("medium", FileType::Regular, 200),
        ];
        
        let mut args = Args::default();
        args.sort_size = true;
        sort_entries(&mut entries, &args);
        
        assert_eq!(entries[0].name, "large");
        assert_eq!(entries[1].name, "medium");
        assert_eq!(entries[2].name, "small");
    }

    #[test]
    fn test_sort_reverse() {
        let mut entries = vec![
            create_test_entry("zebra", FileType::Regular, 100),
            create_test_entry("apple", FileType::Regular, 200),
        ];
        
        let mut args = Args::default();
        args.reverse = true;
        sort_entries(&mut entries, &args);
        
        assert_eq!(entries[0].name, "zebra");
        assert_eq!(entries[1].name, "apple");
    }

    #[test]
    fn test_directories_first() {
        let mut entries = vec![
            create_test_entry("file.txt", FileType::Regular, 100),
            create_test_entry("mydir", FileType::Directory, 4096),
            create_test_entry("another.txt", FileType::Regular, 50),
        ];
        
        let args = Args::default();
        sort_entries(&mut entries, &args);
        
        // Directories should come first
        assert!(entries[0].file_type == FileType::Directory);
    }
}
