use crate::cli::Args;
use crate::types::FileEntry;
use std::fs;
use std::path::Path;

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

    // Sort: directories first, then by name
    sort_entries(&mut file_entries);

    Ok(file_entries)
}

/// Sorts entries: directories first, then alphabetically
fn sort_entries(entries: &mut Vec<FileEntry>) {
    entries.sort_by(|a, b| {
        // First compare by type (directories first)
        let a_is_dir = a.file_type == crate::types::FileType::Directory;
        let b_is_dir = b.file_type == crate::types::FileType::Directory;

        if a_is_dir != b_is_dir {
            return b_is_dir.cmp(&a_is_dir);
        }

        // Then sort alphabetically (case-insensitive)
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
}

/// Extracts the base name of the path for display purposes
pub fn get_dir_name(path: &str) -> String {
    let path = Path::new(path);
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dir_name() {
        assert_eq!(get_dir_name("/home/user/documents"), "documents");
        assert_eq!(get_dir_name("."), ".");
    }
}
