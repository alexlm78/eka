use crate::cli::{Args, SortField};
use crate::glob;
use crate::quoting::quote_for_message;
use crate::types::FileEntry;
use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Recursive directory entry with path information
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub path: PathBuf,
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
                Ok(e) if should_list(&e, args) => file_entries.push(e),
                Ok(_) => {}
                Err(e) => {
                    eprintln!(
                        "eka: cannot access {}: {}",
                        shown(&path.join(dot)),
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
                eprintln!("eka: reading {}: {}", shown(path), io_message(&e));
                *had_error = true;
                continue;
            }
        };
        match FileEntry::with_name(entry.path(), entry.file_name(), args.dereference()) {
            Ok(fe) if should_list(&fe, args) => file_entries.push(fe),
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "eka: cannot access {}: {}",
                    shown(&entry.path()),
                    io_message(&e)
                );
                *had_error = true;
            }
        }
    }

    sort_entries(&mut file_entries, args);
    Ok(file_entries)
}

/// Whether an entry remains after visibility and name-pattern filtering.
fn should_list(entry: &FileEntry, args: &Args) -> bool {
    let name = entry.name.as_bytes();
    if !args.show_hidden() && entry.is_hidden {
        return false;
    }
    if args.ignore_backups && name.ends_with(b"~") {
        return false;
    }
    if args
        .ignore
        .iter()
        .any(|pattern| glob::matches(pattern.as_bytes(), name))
    {
        return false;
    }
    args.show_hidden()
        || !args
            .hide
            .iter()
            .any(|pattern| glob::matches(pattern.as_bytes(), name))
}

/// A directory on the path from the listing root, for cycle detection
struct Ancestor {
    id: (u64, u64),
    parent: Option<Rc<Ancestor>>,
}

impl Ancestor {
    fn contains(node: &Option<Rc<Ancestor>>, id: (u64, u64)) -> bool {
        let mut cur = node.as_deref();
        while let Some(n) = cur {
            if n.id == id {
                return true;
            }
            cur = n.parent.as_deref();
        }
        false
    }
}

/// (device, inode) of what `path` points to
fn dir_id(path: &Path) -> io::Result<(u64, u64)> {
    let m = fs::metadata(path)?;
    Ok((m.dev(), m.ino()))
}

/// Quotes a path for a diagnostic so it cannot inject terminal escapes
pub fn shown(path: &Path) -> String {
    quote_for_message(path.as_os_str().as_bytes())
}

/// Reads a directory and all subdirectories, depth first, in listing order.
/// Iterative, so a deep tree cannot overflow the stack, and a directory that
/// is its own ancestor (possible with -L) is reported and skipped.
pub fn read_directory_recursive(
    path: &Path,
    args: &Args,
    had_error: &mut bool,
) -> io::Result<Vec<DirectoryEntry>> {
    let root = Rc::new(Ancestor {
        id: dir_id(path)?,
        parent: None,
    });
    let mut result = Vec::new();
    let mut stack = vec![(path.to_path_buf(), root)];

    while let Some((dir, chain)) = stack.pop() {
        let entries = match read_directory(&dir, args, had_error) {
            Ok(entries) => entries,
            // The root failing is the caller's error; deeper ones are reported
            Err(e) if result.is_empty() => return Err(e),
            Err(e) => {
                eprintln!(
                    "eka: cannot open directory {}: {}",
                    shown(&dir),
                    io_message(&e)
                );
                *had_error = true;
                continue;
            }
        };
        let subdirs: Vec<PathBuf> = entries
            .iter()
            .filter(|e| e.file_type.is_dir() && e.name != "." && e.name != "..")
            .map(|e| e.path.clone())
            .collect();
        result.push(DirectoryEntry { path: dir, entries });

        // Pushed in reverse so they pop in listing order
        for sub in subdirs.into_iter().rev() {
            match dir_id(&sub) {
                Ok(id) if Ancestor::contains(&Some(chain.clone()), id) => {
                    eprintln!("eka: {}: not listing already-listed directory", shown(&sub));
                    *had_error = true;
                }
                Ok(id) => {
                    let node = Rc::new(Ancestor {
                        id,
                        parent: Some(chain.clone()),
                    });
                    stack.push((sub, node));
                }
                Err(e) => {
                    eprintln!("eka: cannot access {}: {}", shown(&sub), io_message(&e));
                    *had_error = true;
                }
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
                SortField::Version => version_cmp(a.name.as_bytes(), b.name.as_bytes()),
            }
        });
    }
    if args.reverse {
        entries.reverse();
    }
    if args.group_directories_first {
        entries.sort_by_key(|entry| !entry.file_type.is_dir());
    }
}

/// Compares names using runs of ASCII digits as version numbers.
pub fn version_cmp(a: &[u8], b: &[u8]) -> Ordering {
    let mut ai = 0;
    let mut bi = 0;
    while ai < a.len() && bi < b.len() {
        if a[ai].is_ascii_digit() && b[bi].is_ascii_digit() {
            let a_end = a[ai..]
                .iter()
                .position(|byte| !byte.is_ascii_digit())
                .map_or(a.len(), |offset| ai + offset);
            let b_end = b[bi..]
                .iter()
                .position(|byte| !byte.is_ascii_digit())
                .map_or(b.len(), |offset| bi + offset);
            let a_digits = &a[ai..a_end];
            let b_digits = &b[bi..b_end];
            let a_significant = a_digits
                .iter()
                .position(|byte| *byte != b'0')
                .unwrap_or(a_digits.len());
            let b_significant = b_digits
                .iter()
                .position(|byte| *byte != b'0')
                .unwrap_or(b_digits.len());
            let a_value = &a_digits[a_significant..];
            let b_value = &b_digits[b_significant..];
            match a_value
                .len()
                .cmp(&b_value.len())
                .then_with(|| a_value.cmp(b_value))
            {
                Ordering::Equal => {}
                order => return order,
            }
            ai = a_end;
            bi = b_end;
        } else {
            match a[ai].cmp(&b[bi]) {
                Ordering::Equal => {
                    ai += 1;
                    bi += 1;
                }
                order => return order,
            }
        }
    }
    a.cmp(b)
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
    fn test_version_sort_and_comparison() {
        let mut v = vec![entry("file10", 0), entry("file2", 0), entry("file1", 0)];
        let args = Args {
            sort_version: true,
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["file1", "file2", "file10"]);
        assert_eq!(version_cmp(b"a1b", b"a1c"), Ordering::Less);
        assert_eq!(version_cmp(b"file01", b"file1"), Ordering::Less);
        assert_eq!(
            version_cmp(b"x999999999999999999999", b"x1000000000000000000000"),
            Ordering::Less
        );
    }

    #[test]
    fn test_group_directories_first_after_reverse() {
        let mut sub = entry("sub", 0);
        sub.file_type = FileType::Directory;
        let mut another = entry("another", 0);
        another.file_type = FileType::Directory;
        let mut v = vec![entry("a", 0), sub, entry("z", 0), another];
        let args = Args {
            group_directories_first: true,
            reverse: true,
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["sub", "another", "z", "a"]);
    }

    #[test]
    fn test_group_directories_first_keeps_equal_name_order() {
        let mut first = entry("same", 0);
        first.file_type = FileType::Directory;
        first.path = PathBuf::from("first");
        let mut second = entry("same", 0);
        second.file_type = FileType::Directory;
        second.path = PathBuf::from("second");
        let mut v = vec![entry("same", 0), second, entry("other", 0), first];
        let args = Args {
            group_directories_first: true,
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["same", "same", "other", "same"]);
        assert_eq!(v[0].path, PathBuf::from("second"));
        assert_eq!(v[1].path, PathBuf::from("first"));
    }

    #[test]
    fn test_group_directories_first_preserves_directory_order_without_sorting() {
        let mut first = entry("first", 0);
        first.file_type = FileType::Directory;
        let mut second = entry("second", 0);
        second.file_type = FileType::Directory;
        let mut link = entry("link", 0);
        link.file_type = FileType::Symlink;
        let mut v = vec![entry("z", 0), second, link, first, entry("a", 0)];
        let args = Args {
            sort_none: true,
            group_directories_first: true,
            ..Default::default()
        };
        sort_entries(&mut v, &args);
        assert_eq!(names(&v), ["second", "first", "z", "link", "a"]);
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
    fn test_ignore_and_hide_filters() {
        let d = tmp("filters");
        for name in ["a.txt", "a.o", "keep", "x~", ".hidden"] {
            fs::write(d.join(name), b"").unwrap();
        }
        let ignored = Args {
            ignore: vec!["*.txt".to_string(), ".hidden".to_string()],
            hide: vec!["a*".to_string()],
            ignore_backups: true,
            ..Default::default()
        };
        assert_eq!(list(&d, &ignored), ["keep"]);
        let all = Args {
            all: true,
            ignore: vec!["*.txt".to_string(), ".hidden".to_string()],
            hide: vec!["a*".to_string()],
            ignore_backups: true,
            ..Default::default()
        };
        assert_eq!(list(&d, &all), [".", "..", "a.o", "keep"]);
        let almost_all = Args {
            almost_all: true,
            ..ignored
        };
        assert_eq!(list(&d, &almost_all), ["a.o", "keep"]);
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

    #[test]
    fn test_recursive_order_is_depth_first_in_listing_order() {
        let d = tmp("rorder");
        for p in ["a/x", "a/y", "b", "c/z"] {
            fs::create_dir_all(d.join(p)).unwrap();
        }
        let mut err = false;
        let args = Args {
            recursive: true,
            ..Default::default()
        };
        let dirs = read_directory_recursive(&d, &args, &mut err).unwrap();
        let rel: Vec<String> = dirs
            .iter()
            .map(|e| e.path.strip_prefix(&d).unwrap().display().to_string())
            .collect();
        assert_eq!(rel, ["", "a", "a/x", "a/y", "b", "c", "c/z"]);
        assert!(!err);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_recursive_dereference_cycle_is_reported_once_and_terminates() {
        let d = tmp("rcycle");
        fs::create_dir(d.join("sub")).unwrap();
        std::os::unix::fs::symlink("..", d.join("sub/up")).unwrap();
        std::os::unix::fs::symlink(".", d.join("self")).unwrap();
        let mut err = false;
        let args = Args {
            recursive: true,
            deref_all: true,
            ..Default::default()
        };
        let dirs = read_directory_recursive(&d, &args, &mut err).unwrap();
        // the root and sub are listed; both links point back to an ancestor
        assert_eq!(dirs.len(), 2);
        assert!(err);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_recursive_dereference_lists_a_linked_directory_listed_twice() {
        // two links to one directory are not a cycle: GNU lists it under both
        let d = tmp("rtwice");
        fs::create_dir_all(d.join("real")).unwrap();
        std::os::unix::fs::symlink("real", d.join("l1")).unwrap();
        std::os::unix::fs::symlink("real", d.join("l2")).unwrap();
        let mut err = false;
        let args = Args {
            recursive: true,
            deref_all: true,
            ..Default::default()
        };
        let dirs = read_directory_recursive(&d, &args, &mut err).unwrap();
        assert_eq!(dirs.len(), 4);
        assert!(!err);
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn test_recursive_root_failure_is_returned_not_reported() {
        let mut err = false;
        let args = Args {
            recursive: true,
            ..Default::default()
        };
        assert!(read_directory_recursive(Path::new("/nonexistent-eka"), &args, &mut err).is_err());
    }
}
