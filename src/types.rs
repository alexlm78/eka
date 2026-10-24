use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::cli::TimeField;

/// Categorized file types for color coding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Directory,
    Executable,
    Symlink,
    BrokenSymlink,
    Fifo,
    Socket,
    CharDevice,
    BlockDevice,
    Image,
    Video,
    Audio,
    Compressed,
    Regular,
}

impl FileType {
    /// Classifies an entry from its own (non-followed) metadata and name
    pub fn classify(name: &[u8], metadata: &fs::Metadata) -> Self {
        let ft = metadata.file_type();
        if ft.is_dir() {
            return FileType::Directory;
        }
        if ft.is_symlink() {
            return FileType::Symlink;
        }
        if ft.is_fifo() {
            return FileType::Fifo;
        }
        if ft.is_socket() {
            return FileType::Socket;
        }
        if ft.is_char_device() {
            return FileType::CharDevice;
        }
        if ft.is_block_device() {
            return FileType::BlockDevice;
        }
        if metadata.mode() & 0o111 != 0 {
            return FileType::Executable;
        }
        Self::from_extension(name)
    }

    fn from_extension(name: &[u8]) -> Self {
        let ext = match Path::new(std::ffi::OsStr::from_bytes(name)).extension() {
            Some(e) => e.as_bytes().to_ascii_lowercase(),
            None => return FileType::Regular,
        };
        match ext.as_slice() {
            b"jpg" | b"jpeg" | b"png" | b"gif" | b"bmp" | b"svg" | b"webp" | b"tiff" => {
                FileType::Image
            }
            b"mp4" | b"avi" | b"mkv" | b"mov" | b"wmv" | b"flv" | b"webm" | b"m4v" | b"mpeg"
            | b"mpg" => FileType::Video,
            b"mp3" | b"wav" | b"flac" | b"aac" | b"ogg" | b"wma" | b"m4a" | b"aiff" => {
                FileType::Audio
            }
            b"zip" | b"tar" | b"gz" | b"bz2" | b"7z" | b"rar" | b"xz" | b"tgz" | b"cab" => {
                FileType::Compressed
            }
            _ => FileType::Regular,
        }
    }

    /// Returns true for directories (symlinks to directories are not followed)
    pub fn is_dir(&self) -> bool {
        *self == FileType::Directory
    }

    /// Returns the classification indicator for this file type (-F)
    pub fn indicator(&self) -> &'static str {
        match self {
            FileType::Directory => "/",
            FileType::Executable => "*",
            FileType::Symlink | FileType::BrokenSymlink => "@",
            FileType::Fifo => "|",
            FileType::Socket => "=",
            _ => "",
        }
    }
}

/// Represents a file entry in a directory listing
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Raw name bytes as stored by the filesystem (may not be UTF-8)
    pub name: OsString,
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub permissions: String,
    pub modified: SystemTime,
    pub accessed: SystemTime,
    pub changed: SystemTime,
    pub is_hidden: bool,
    pub symlink_target: Option<OsString>,
    pub inode: u64,
    pub nlink: u64,
    // Consumed by the long-format/total work in the next phases
    #[allow(dead_code)]
    pub uid: u32,
    #[allow(dead_code)]
    pub gid: u32,
    /// Allocated size in 512-byte blocks
    #[allow(dead_code)]
    pub blocks: u64,
    /// A symlink whose target is a directory (grouped with directories by
    /// --group-directories-first, as GNU ls does)
    pub links_to_dir: bool,
}

fn unix_time(secs: i64, nsecs: i64) -> SystemTime {
    // Checked, so a bogus timestamp from the filesystem cannot panic the listing
    let nanos = Duration::from_nanos(nsecs.clamp(0, 999_999_999) as u64);
    let shifted = if secs >= 0 {
        SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(secs as u64))
    } else {
        SystemTime::UNIX_EPOCH.checked_sub(Duration::from_secs(secs.unsigned_abs()))
    };
    shifted
        .and_then(|t| t.checked_add(nanos))
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

impl FileEntry {
    /// Creates an entry from a path, using the final path component as name
    #[cfg(test)]
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let name = match path.file_name() {
            Some(n) => n.to_os_string(),
            None => path.clone().into_os_string(),
        };
        Self::with_name(path, name, false)
    }

    /// Creates an entry for `path` shown under `name` (used for "." and "..").
    /// With `deref`, a symlink reports its target's metadata; broken links
    /// fall back to the link itself.
    pub fn with_name(path: PathBuf, name: OsString, deref: bool) -> std::io::Result<Self> {
        // lstat by default, so broken links are still listed
        let mut metadata = fs::symlink_metadata(&path)?;
        if deref
            && metadata.file_type().is_symlink()
            && let Ok(target_meta) = fs::metadata(&path)
        {
            metadata = target_meta;
        }
        let bytes = name.as_bytes();
        let is_hidden = bytes.first() == Some(&b'.');

        let is_symlink = metadata.file_type().is_symlink();
        let mut file_type = FileType::classify(bytes, &metadata);
        let mut symlink_target = None;
        let mut links_to_dir = false;
        if is_symlink {
            symlink_target = fs::read_link(&path).ok().map(|p| p.into_os_string());
            match fs::metadata(&path) {
                Err(_) => file_type = FileType::BrokenSymlink,
                Ok(target) => links_to_dir = target.is_dir(),
            }
        }

        Ok(Self {
            name,
            file_type,
            size: metadata.len(),
            permissions: format_permissions(&metadata),
            modified: unix_time(metadata.mtime(), metadata.mtime_nsec()),
            accessed: unix_time(metadata.atime(), metadata.atime_nsec()),
            changed: unix_time(metadata.ctime(), metadata.ctime_nsec()),
            is_hidden,
            symlink_target,
            inode: metadata.ino(),
            nlink: metadata.nlink(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            blocks: metadata.blocks(),
            links_to_dir,
            path,
        })
    }

    /// Lossy name for tests and messages; output paths write the raw bytes
    #[cfg(test)]
    pub fn display_name(&self) -> std::borrow::Cow<'_, str> {
        self.name.to_string_lossy()
    }

    /// Timestamp selected by -u/-c (modification time by default)
    pub fn time(&self, field: TimeField) -> SystemTime {
        match field {
            TimeField::Modified => self.modified,
            TimeField::Accessed => self.accessed,
            TimeField::Changed => self.changed,
        }
    }
}

/// Formats the mode string like `ls -l` (type char, rwx triplets, setuid/setgid/sticky)
fn format_permissions(metadata: &fs::Metadata) -> String {
    let mode = metadata.mode();
    let ft = metadata.file_type();
    let kind = if ft.is_dir() {
        'd'
    } else if ft.is_symlink() {
        'l'
    } else if ft.is_char_device() {
        'c'
    } else if ft.is_block_device() {
        'b'
    } else if ft.is_fifo() {
        'p'
    } else if ft.is_socket() {
        's'
    } else {
        '-'
    };

    let triplet = |shift: u32, special: bool, special_ch: char| {
        let bits = (mode >> shift) & 0o7;
        let x = bits & 1 != 0;
        [
            if bits & 4 != 0 { 'r' } else { '-' },
            if bits & 2 != 0 { 'w' } else { '-' },
            match (x, special) {
                (true, true) => special_ch,
                (false, true) => special_ch.to_ascii_uppercase(),
                (true, false) => 'x',
                (false, false) => '-',
            },
        ]
    };

    let mut s = String::with_capacity(10);
    s.push(kind);
    s.extend(triplet(6, mode & 0o4000 != 0, 's'));
    s.extend(triplet(3, mode & 0o2000 != 0, 's'));
    s.extend(triplet(0, mode & 0o1000 != 0, 't'));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("eka-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_file_type_indicator() {
        assert_eq!(FileType::Directory.indicator(), "/");
        assert_eq!(FileType::Executable.indicator(), "*");
        assert_eq!(FileType::Symlink.indicator(), "@");
        assert_eq!(FileType::Fifo.indicator(), "|");
        assert_eq!(FileType::Socket.indicator(), "=");
        assert_eq!(FileType::Regular.indicator(), "");
    }

    #[test]
    fn test_extension_detection() {
        assert_eq!(FileType::from_extension(b"a.PNG"), FileType::Image);
        assert_eq!(FileType::from_extension(b"a.mkv"), FileType::Video);
        assert_eq!(FileType::from_extension(b"a.flac"), FileType::Audio);
        assert_eq!(FileType::from_extension(b"a.tar"), FileType::Compressed);
        assert_eq!(FileType::from_extension(b"png"), FileType::Regular);
    }

    #[test]
    fn test_symlink_and_broken_symlink() {
        let d = tmp("symlink");
        fs::write(d.join("real"), b"x").unwrap();
        symlink("real", d.join("ok")).unwrap();
        symlink("missing", d.join("bad")).unwrap();

        let ok = FileEntry::from_path(d.join("ok")).unwrap();
        assert!(ok.symlink_target.is_some());
        assert_eq!(ok.file_type, FileType::Symlink);
        assert!(ok.permissions.starts_with('l'));
        assert_eq!(ok.symlink_target.as_deref(), Some("real".as_ref()));

        let bad = FileEntry::from_path(d.join("bad")).unwrap();
        assert_eq!(bad.file_type, FileType::BrokenSymlink);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_deref_reports_target() {
        let d = tmp("deref");
        fs::write(d.join("real"), b"hello").unwrap();
        symlink("real", d.join("ln")).unwrap();
        symlink("missing", d.join("bad")).unwrap();

        let plain = FileEntry::with_name(d.join("ln"), "ln".into(), false).unwrap();
        assert!(plain.symlink_target.is_some());

        let deref = FileEntry::with_name(d.join("ln"), "ln".into(), true).unwrap();
        assert!(deref.symlink_target.is_none());
        assert_eq!(deref.size, 5);
        assert!(deref.permissions.starts_with('-'));

        // broken link falls back to the link itself
        let bad = FileEntry::with_name(d.join("bad"), "bad".into(), true).unwrap();
        assert_eq!(bad.file_type, FileType::BrokenSymlink);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_special_permission_bits() {
        let d = tmp("perm");
        let f = d.join("f");
        fs::write(&f, b"x").unwrap();
        fs::set_permissions(&f, fs::Permissions::from_mode(0o4755)).unwrap();
        assert_eq!(
            FileEntry::from_path(f.clone()).unwrap().permissions,
            "-rwsr-xr-x"
        );
        fs::set_permissions(&f, fs::Permissions::from_mode(0o1644)).unwrap();
        assert_eq!(
            FileEntry::from_path(f.clone()).unwrap().permissions,
            "-rw-r--r-T"
        );
        fs::set_permissions(&f, fs::Permissions::from_mode(0o2751)).unwrap();
        assert_eq!(FileEntry::from_path(f).unwrap().permissions, "-rwxr-s--x");
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_non_utf8_name_preserved() {
        use std::ffi::OsStr;
        let d = tmp("nonutf8");
        let name = OsStr::from_bytes(b"bad\xff.txt");
        // APFS and other filesystems reject non-UTF-8 names; nothing to test there
        if fs::write(d.join(name), b"x").is_err() {
            fs::remove_dir_all(d).unwrap();
            return;
        }
        let e = FileEntry::from_path(d.join(name)).unwrap();
        assert_eq!(e.name.as_bytes(), b"bad\xff.txt");
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_ctime_differs_from_mtime_source() {
        let d = tmp("ctime");
        let f = d.join("f");
        fs::write(&f, b"x").unwrap();
        let e = FileEntry::from_path(f).unwrap();
        assert!(e.changed > SystemTime::UNIX_EPOCH);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_unix_time_never_panics_on_extreme_values() {
        for secs in [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX] {
            for nsecs in [i64::MIN, -1, 0, 999_999_999, 1_000_000_000, i64::MAX] {
                let _ = unix_time(secs, nsecs);
            }
        }
        assert_eq!(unix_time(0, 0), SystemTime::UNIX_EPOCH);
        assert_eq!(
            unix_time(-5, 500_000_000),
            SystemTime::UNIX_EPOCH - Duration::from_millis(4_500)
        );
        assert_eq!(
            unix_time(10, 1),
            SystemTime::UNIX_EPOCH + Duration::new(10, 1)
        );
    }
}
