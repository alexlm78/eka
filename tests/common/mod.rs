//! Shared helpers for the integration tests.
//!
//! Every test builds its own small tree in a private temporary directory, with
//! fixed timestamps, and runs the real `eka` binary against it.
#![allow(dead_code)]

use std::fs::{self, File, FileTimes};
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// 2020-01-01 12:00:00 UTC, the base of every fixed timestamp
const BASE: i64 = 1_577_880_000;

/// Seconds since the epoch for `n` days after 2020-01-01 12:00 UTC
pub fn day(n: i64) -> i64 {
    BASE + n * 86_400
}

fn at(secs: i64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64)
}

/// A private temporary directory that is removed when dropped
pub struct Fixture {
    root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "eka-it-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// Creates a file of `len` bytes with mode 0644 and the given mtime
    pub fn file(&self, rel: &str, len: usize, mtime: i64) -> &Self {
        let path = self.path(rel);
        let mut file = File::create(&path).unwrap();
        file.write_all(&vec![b'x'; len]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        set_times(&path, mtime, mtime);
        self
    }

    /// Like `file`, with a distinct access time
    pub fn file_at(&self, rel: &str, len: usize, atime: i64, mtime: i64) -> &Self {
        self.file(rel, len, mtime);
        set_times(&self.path(rel), atime, mtime);
        self
    }

    pub fn dir(&self, rel: &str) -> &Self {
        fs::create_dir_all(self.path(rel)).unwrap();
        self
    }

    pub fn link(&self, target: &str, rel: &str) -> &Self {
        symlink(target, self.path(rel)).unwrap();
        self
    }

    pub fn mode(&self, rel: &str, mode: u32) -> &Self {
        fs::set_permissions(self.path(rel), fs::Permissions::from_mode(mode)).unwrap();
        self
    }

    /// Fixes a directory's mtime; creating children changes it, so call it last
    pub fn dir_time(&self, rel: &str, mtime: i64) -> &Self {
        set_times(&self.path(rel), mtime, mtime);
        self
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // A test may leave an unreadable directory behind
        make_removable(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn make_removable(dir: &Path) {
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                make_removable(&entry.path());
            }
        }
    }
}

fn set_times(path: &Path, atime: i64, mtime: i64) {
    let times = FileTimes::new()
        .set_accessed(at(atime))
        .set_modified(at(mtime));
    File::open(path).unwrap().set_times(times).unwrap();
}

/// The regular tree most tests use. Visible names in byte order:
/// a.txt a1 a10 a2 backup~ big.bin c.rs empty-dir link-broken link-dir
/// link-ok sub "with space"
pub fn standard_tree() -> Fixture {
    let f = Fixture::new();
    f.file("a.txt", 6, day(1))
        .file("a1", 1, day(2))
        .file("a10", 10, day(3))
        .file("a2", 2, day(4))
        .file("backup~", 1, day(5))
        .file("big.bin", 3000, day(6))
        .mode("big.bin", 0o755)
        .file("c.rs", 3, day(7))
        .file("with space", 4, day(8))
        .file(".hidden", 1, day(9))
        .dir(".hdir")
        .file(".hdir/inside", 1, day(10))
        .dir("empty-dir")
        .dir("sub")
        .file("sub/inner.txt", 2, day(11))
        .dir("sub/sub2")
        .file("sub/sub2/deep.txt", 3, day(12))
        .link("a.txt", "link-ok")
        .link("sub", "link-dir")
        .link("missing", "link-broken");
    for dir in ["sub/sub2", "sub", ".hdir", "empty-dir"] {
        f.dir_time(dir, day(20));
    }
    f.dir_time(".", day(20));
    f
}

/// Four regular files with distinct sizes, modification and access times
pub fn flat_tree() -> Fixture {
    let f = Fixture::new();
    f.file_at("f_big", 500, day(50), day(60))
        .file_at("f_mid", 50, day(200), day(0))
        .file_at("f_mid2", 50, day(10), day(31))
        .file_at("f_small", 5, day(100), day(121));
    f.dir_time(".", day(130));
    f
}

fn clean_command(program: &str, dir: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("LC_ALL", "C")
        .env("TZ", "UTC");
    cmd
}

pub const EKA: &str = env!("CARGO_BIN_EXE_eka");

/// Runs eka inside the fixture root
pub fn eka(f: &Fixture, args: &[&str]) -> Output {
    eka_env(f, args, &[])
}

/// `ls -l` prints a marker after the mode when the file has extended
/// attributes (`@`, and macOS tags every file with com.apple.provenance), a
/// POSIX ACL (`+`) or an SELinux context (`.`). Expected listings are written
/// without it, so this takes it out. Tests of the markers themselves run the
/// binary directly.
pub fn strip_attribute_marker(bytes: Vec<u8>) -> Vec<u8> {
    let is_mode = |m: &[u8]| {
        m.len() == 10
            && b"-dlcbps".contains(&m[0])
            && m[1..].iter().all(|b| b"rwxsStT-".contains(b))
    };
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        if b"@.+".contains(&b)
            && i >= 10
            && is_mode(&bytes[i - 10..i])
            && bytes.get(i + 1) == Some(&b' ')
        {
            continue;
        }
        out.push(b);
    }
    out
}

pub fn eka_env(f: &Fixture, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = clean_command(EKA, f.root());
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut output = cmd.output().unwrap();
    output.stdout = strip_attribute_marker(output.stdout);
    output
}

/// The ls to compare against: `$EKA_TEST_LS`, or `/bin/ls`
pub fn ls_path() -> String {
    std::env::var("EKA_TEST_LS").unwrap_or_else(|_| "/bin/ls".to_owned())
}

/// Runs the reference ls inside the fixture root
pub fn sys_ls(f: &Fixture, args: &[&str]) -> Output {
    let mut cmd = clean_command(&ls_path(), f.root());
    cmd.args(args);
    let mut output = cmd.output().unwrap();
    output.stdout = strip_attribute_marker(output.stdout);
    output
}

pub fn has_system_ls() -> bool {
    Path::new(&ls_path()).exists()
}

/// GNU coreutils answers `--version`; the BSD ls rejects it
pub fn system_ls_is_gnu() -> bool {
    Command::new(ls_path())
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// stdout of a command that must have succeeded
pub fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "exit {:?}, stderr: {:?}",
        output.status.code(),
        text(&output.stderr)
    );
    text(&output.stdout)
}

/// stdout of a successful `eka -1 ARGS` as a list of lines
pub fn names(f: &Fixture, args: &[&str]) -> Vec<String> {
    let mut all = vec!["-1"];
    all.extend_from_slice(args);
    ok(&eka(f, &all)).lines().map(str::to_owned).collect()
}

pub fn strs(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// Current user and group names
pub fn user_and_group() -> (String, String) {
    let id = |flag: &str| {
        let out = Command::new("id").arg(flag).output().unwrap();
        text(&out.stdout).trim().to_owned()
    };
    (id("-un"), id("-gn"))
}

pub fn running_as_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .is_ok_and(|o| text(&o.stdout).trim() == "0")
}
