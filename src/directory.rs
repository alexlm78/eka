use crate::cli::{Args, SortField};
use crate::types::FileEntry;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
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
pub fn read_directory(
    path: &Path,
    args: &Args,
    had_error: &mut bool,
) -> io::Result<Vec<FileEntry>> {
    let mut file_entries: Vec<FileEntry> = Vec::new();

    if args.all {
        for dot in [".", ".."] {
            match FileEntry::with_name(path.join(dot), OsString::from(dot), false) {
                Ok(e) => file_entries.push(e),
                Err(e) => {
                    eprintln!(
                        "eka: cannot access '{}/{}': {}",
                        path.display(),
                        dot,
                        io_message(&e)
                    );
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
        match FileEntry::with_name(entry.path(), entry.file_name(), args.dereference()) {
            Ok(fe) if args.show_hidden() || !fe.is_hidden => file_entries.push(fe),
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "eka: cannot access '{}': {}",
                    entry.path().display(),
                    io_message(&e)
                );
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
                eprintln!(
                    "eka: cannot open directory '{}': {}",
                    sub.display(),
                    io_message(&e)
                );
                *had_error = true;
            }
        }
    }
    Ok(result)
}

/// Sorts entries per the CLI options. Names compare bytewise (C locale);
/// size and time ties fall back to name, and -r reverses the whole order.
pub fn sort_entries(entries: &mut [FileEntry], args: &Args) {
    let field = args.sort_field();
    let time = args.time_field();

    if field != SortField::None {
        entries.sort_by(|a, b| {
            let by_name = || a.name.as_bytes().cmp(b.name.as_bytes());
            match field {
                SortField::Name | SortField::None => by_name(),
                SortField::Size => b.size.cmp(&a.size).then_with(by_name),
                SortField::Time => b.time(time).cmp(&a.time(time)).then_with(by_name),
                SortField::Extension => extension(a).cmp(extension(b)).then_with(by_name),
            }
        });
    }
    if args.reverse {
        entries.reverse();
    }
}

/// Bytes after the last dot of the name (empty when there is none)
fn extension(e: &FileEntry) -> &[u8] {
    Path::new(&e.name)
        .extension()
        .map_or(&[][..], |x| x.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::time::{Duration, SystemTime};

    fn entry(name: &str, size: u64) -> FileEntry {
        FileEntry {
            name: name.into(),
            path: PathBuf::from(name),
            file_type: FileType::Regular,
            size,
            permissions: "-rw-r--r--".to_string(),
            modified: SystemTime::UNIX_EPOCH,
            accessed: SystemTime::UNIX_EPOCH,
            changed: SystemTime::UNIX_EPOCH,
            is_hidden: name.starts_with('.'),
            symlink_target: None,
            inode: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            blocks: 0,
        }
    }

    fn names(entries: &[FileEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|e| e.display_name().into_owned())
            .collect()
    }

    #[test]
    fn test_sort_by_name_is_bytewise() {
        let mut v = vec![entry("b", 0), entry("a", 0), entry("B", 0), entry("_x", 0)];
        sort_entries(&mut v, &Args::default());
        assert_eq!(names(&v), ["B", "_x", "a", "b"]);
    }

    #[test]
    fn test_directories_are_not_grouped_first() {
        let mut d = entry("a_dir", 4096);
        d.file_type = FileType::Directory;
        let mut v = vec![entry("b.txt", 1), d];
        sort_entries(&mut v, &Args::default());
        assert_eq!(names(&v), ["a_dir", "b.txt"]);
    }

    #[test]
    fn test_sort_by_size_desc_ties_by_name() {
        let mut v = vec![entry("z", 10), entry("a", 10), entry("big", 100)];
        let args = Args {
            sort_size: true,
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["big", "a", "z"]);
    }

    #[test]
    fn test_sort_by_time_newest_first_and_selected_field() {
        let mut old = entry("old", 0);
        let mut new = entry("new", 0);
        old.modified = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        new.modified = SystemTime::UNIX_EPOCH + Duration::from_secs(20);
        // access times are the opposite order
        old.accessed = SystemTime::UNIX_EPOCH + Duration::from_secs(99);
        new.accessed = SystemTime::UNIX_EPOCH + Duration::from_secs(1);

        let mut v = vec![old.clone(), new.clone()];
        sort_entries(
            &mut v,
            &Args {
                sort_time: true,
                ..Default::default()
            },
        );
        assert_eq!(names(&v), ["new", "old"]);

        let mut v = vec![new, old];
        sort_entries(
            &mut v,
            &Args {
                sort_time: true,
                sort_access: true,
                ..Default::default()
            },
        );
        assert_eq!(names(&v), ["old", "new"]);
    }

    #[test]
    fn test_sort_by_extension() {
        let mut v = vec![
            entry("b.rs", 0),
            entry("a.txt", 0),
            entry("c.rs", 0),
            entry("noext", 0),
        ];
        let args = Args {
            sort: Some(SortField::Extension),
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["noext", "b.rs", "c.rs", "a.txt"]);
    }

    #[test]
    fn test_reverse_reverses_ties_too() {
        let mut v = vec![entry("a", 10), entry("b", 10), entry("c", 50)];
        let args = Args {
            sort_size: true,
            reverse: true,
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["b", "a", "c"]);
    }

    #[test]
    fn test_sort_none_keeps_order() {
        let mut v = vec![entry("z", 0), entry("a", 0)];
        let args = Args {
            sort: Some(SortField::None),
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["z", "a"]);
        let args = Args {
            sort: Some(SortField::None),
            reverse: true,
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["a", "z"]);
    }

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("eka-dir-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn list(dir: &Path, args: &Args) -> Vec<String> {
        let mut err = false;
        names(&read_directory(dir, args, &mut err).unwrap())
    }

    #[test]
    fn test_all_vs_almost_all_vs_default() {
        let d = tmp("hidden");
        fs::write(d.join(".h"), b"").unwrap();
        fs::write(d.join("v"), b"").unwrap();

        assert_eq!(list(&d, &Args::default()), ["v"]);
        assert_eq!(
            list(
                &d,
                &Args {
                    almost_all: true,
                    ..Default::default()
                }
            ),
            [".h", "v"]
        );
        assert_eq!(
            list(
                &d,
                &Args {
                    all: true,
                    ..Default::default()
                }
            ),
            [".", "..", ".h", "v"]
        );
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_recursive_does_not_follow_symlink_cycles() {
        let d = tmp("cycle");
        fs::create_dir(d.join("sub")).unwrap();
        std::os::unix::fs::symlink("..", d.join("sub/up")).unwrap();
        let mut err = false;
        let args = Args {
            recursive: true,
            ..Default::default()
        };
        let dirs = read_directory_recursive(&d, &args, &mut err).unwrap();
        assert_eq!(dirs.len(), 2);
        assert!(!err);
        fs::remove_dir_all(d).unwrap();
    }
}
