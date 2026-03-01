use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Categorized file types for color coding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Directory,
    Executable,
    Symlink,
    Image,
    Video,
    Audio,
    Compressed,
    Regular,
}

impl FileType {
    /// Determines the file type based on its name and metadata
    pub fn from_path(path: &PathBuf, metadata: &fs::Metadata) -> Self {
        // First check if it's a directory
        if metadata.is_dir() {
            return FileType::Directory;
        }

        // Check if it's a symlink
        if metadata.file_type().is_symlink() {
            return FileType::Symlink;
        }

        // Check if it's executable
        if metadata.permissions().readonly() == false {
            // Only executable if it has execute permission
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();
                if mode & 0o111 != 0 {
                    return FileType::Executable;
                }
            }
        }

        // Determine by extension
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy().to_lowercase();
            
            // Images
            if matches!(name_str.as_str(), 
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff"
            ) {
                return FileType::Image;
            }

            // Videos
            if matches!(name_str.as_str(),
                "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v"
            ) {
                return FileType::Video;
            }

            // Audio
            if matches!(name_str.as_str(),
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a"
            ) {
                return FileType::Audio;
            }

            // Compressed files
            if matches!(name_str.as_str(),
                "zip" | "tar" | "gz" | "bz2" | "7z" | "rar" | "xz" | "tgz"
            ) {
                return FileType::Compressed;
            }
        }

        FileType::Regular
    }
}

/// Represents a file entry in a directory listing
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub permissions: String,
    pub modified: SystemTime,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
}

impl FileEntry {
    /// Creates a new file entry from a path
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_hidden = name.starts_with('.');
        
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return None,
        };

        let mut is_symlink = false;
        let mut symlink_target = None;

        // If it's a symlink, obtain the target
        if metadata.file_type().is_symlink() {
            is_symlink = true;
            symlink_target = fs::read_link(&path)
                .ok()
                .map(|p| p.to_string_lossy().to_string());
        }

        let file_type = FileType::from_path(&path, &metadata);
        
        // Get permissions in rw-r--r-- format
        let permissions = Self::format_permissions(&metadata);

        Some(Self {
            name,
            path,
            file_type,
            size: metadata.len(),
            permissions,
            modified: metadata.modified().ok().unwrap_or(SystemTime::UNIX_EPOCH),
            is_hidden,
            is_symlink,
            symlink_target,
        })
    }

    /// Formats permissions in Unix style (rwx)
    #[cfg(unix)]
    fn format_permissions(metadata: &fs::Metadata) -> String {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        
        let file_type = if metadata.is_dir() { 'd' } else { '-' };
        
        let user = format!(
            "{}{}{}",
            if mode & 0o400 != 0 { 'r' } else { '-' },
            if mode & 0o200 != 0 { 'w' } else { '-' },
            if mode & 0o100 != 0 { 'x' } else { '-' }
        );
        
        let group = format!(
            "{}{}{}",
            if mode & 0o040 != 0 { 'r' } else { '-' },
            if mode & 0o020 != 0 { 'w' } else { '-' },
            if mode & 0o010 != 0 { 'x' } else { '-' }
        );
        
        let other = format!(
            "{}{}{}",
            if mode & 0o004 != 0 { 'r' } else { '-' },
            if mode & 0o002 != 0 { 'w' } else { '-' },
            if mode & 0o001 != 0 { 'x' } else { '-' }
        );

        format!("{}{}{}{}", file_type, user, group, other)
    }

    #[cfg(not(unix))]
    fn format_permissions(_metadata: &fs::Metadata) -> String {
        "-rw-r--r--".to_string()
    }
}
