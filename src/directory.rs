use crate::cli::{Args, SortField};
use crate::types::FileEntry;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Recursive directory entry with path information
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

/// Strips the " (os error N)" suffix so messages read like ls
pub fn io_message(e: &io::Error) -> String {
    let s = e.to_string();
    match s.find(" (os error") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}

/// Reads a directory and returns filtered, sorted entries.
/// Per-entry failures are reported on stderr and set `had_error`.
pub fn read_directory(path: &Path, args: &Args, had_error: &mut bool) -> io::Result<Vec<FileEntry>> {
    let mut file_entries: Vec<FileEntry> = Vec::new();

    if args.all {
        for dot in [".", ".."] {
            match FileEntry::with_name(path.join(dot), OsString::from(dot)) {
                Ok(e) => file_entries.push(e),
                Err(e) => {
                    eprintln!("eka: cannot access '{}/{}': {}", path.display(), dot, io_message(&e));
                    *had_error = true;
                }
            }
        }
    }

    for entry in fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("eka: reading '{}': {}", path.display(), io_message(&e));
                *had_error = true;
                continue;
            }
        };
        match FileEntry::with_name(entry.path(), entry.file_name()) {
            Ok(fe) if args.all || !fe.is_hidden => file_entries.push(fe),
            Ok(_) => {}
            Err(e) => {
                eprintln!("eka: cannot access '{}': {}", entry.path().display(), io_message(&e));
                *had_error = true;
            }
        }
    }

    sort_entries(&mut file_entries, args);
    Ok(file_entries)
}

/// Reads a directory and all subdirectories (symlinks are not followed, so no cycles)
pub fn read_directory_recursive(
    path: &Path,
    args: &Args,
    had_error: &mut bool,
) -> io::Result<Vec<DirectoryEntry>> {
    let mut result = Vec::new();
    let entries = read_directory(path, args, had_error)?;
    let subdirs: Vec<PathBuf> = entries
        .iter()
        .filter(|e| e.file_type.is_dir() && e.name != "." && e.name != "..")
        .map(|e| e.path.clone())
        .collect();
    result.push(DirectoryEntry {
        path: path.display().to_string(),
        entries,
    });

    for sub in subdirs {
        match read_directory_recursive(&sub, args, had_error) {
            Ok(v) => result.extend(v),
            Err(e) => {
                eprintln!("eka: cannot open directory '{}': {}", sub.display(), io_message(&e));
                *had_error = true;
            }
        }
    }
    Ok(result)
}

/// Sorts entries based on CLI arguments
pub fn sort_entries(entries: &mut [FileEntry], args: &Args) {
    let sort_field = args.get_sort_field();
    let reverse = args.reverse;
    
    entries.sort_by(|a, b| {
        // First compare by type (directories first), but only for name sorting
        if sort_field == SortField::Name {
            let a_is_dir = a.file_type.is_dir();
            let b_is_dir = b.file_type.is_dir();
            
            if a_is_dir != b_is_dir {
                let cmp = b_is_dir.cmp(&a_is_dir);
                return if reverse { cmp.reverse() } else { cmp };
            }
        }

        // Sort by the selected field
        let cmp = match sort_field {
            SortField::Name => {
                a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn create_test_entry(name: &str, file_type: FileType, size: u64) -> FileEntry {
        FileEntry {
            name: name.into(),
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
            inode: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            blocks: 0,
        }
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
