//! Compares eka with the system `ls` on the same fixture trees.
//!
//! The reference is `/bin/ls`, or `$EKA_TEST_LS` (for example a GNU `gls` on
//! macOS). The BSD and GNU programs differ in places, so the flags that only
//! GNU has are checked only when the reference is GNU, and the long format is
//! compared exactly with GNU and after normalizing whitespace and the BSD
//! extras (`@`/`+` after the mode, 512-byte `total`) with BSD.

mod common;

use common::*;

/// Runs both programs on every argument list and reports every difference
fn assert_same(f: &Fixture, cases: &[&[&str]]) {
    let mut diffs = Vec::new();
    for args in cases {
        let (mine, theirs) = (eka(f, args), sys_ls(f, args));
        if mine.stdout != theirs.stdout {
            diffs.push(format!(
                "{args:?}\n  eka: {:?}\n  ls:  {:?}",
                text(&mine.stdout),
                text(&theirs.stdout)
            ));
        }
        if mine.status.success() != theirs.status.success() {
            diffs.push(format!(
                "{args:?}: exit eka {:?} vs ls {:?}",
                mine.status.code(),
                theirs.status.code()
            ));
        }
    }
    assert!(
        diffs.is_empty(),
        "differences from {}:\n{}",
        ls_path(),
        diffs.join("\n")
    );
}

/// Like `assert_same` for the column layouts. Older GNU coreutils (Ubuntu's)
/// need the columns to fit strictly inside the width (`total < width`), newer
/// ones accept an exact fit (`total <= width`), and eka follows the newer
/// rule. So the reference at width N may equal eka at width N or at N - 1.
/// Nothing else is allowed to differ.
fn assert_same_columns(f: &Fixture, cases: &[&[&str]]) {
    let mut diffs = Vec::new();
    for args in cases {
        let theirs = sys_ls(f, args);
        if eka(f, args).stdout == theirs.stdout {
            continue;
        }
        let mut narrower: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        if let Some(i) = narrower.iter().position(|a| a == "-w")
            && let Some(width) = narrower.get(i + 1).and_then(|w| w.parse::<usize>().ok())
            && width > 1
        {
            narrower[i + 1] = (width - 1).to_string();
        }
        let refs: Vec<&str> = narrower.iter().map(String::as_str).collect();
        if eka(f, &refs).stdout == theirs.stdout {
            continue;
        }
        diffs.push(format!(
            "{args:?}\n  eka: {:?}\n  ls:  {:?}",
            text(&eka(f, args).stdout),
            text(&theirs.stdout)
        ));
    }
    assert!(
        diffs.is_empty(),
        "differences from {}:\n{}",
        ls_path(),
        diffs.join("\n")
    );
}

macro_rules! need_ls {
    () => {
        if !has_system_ls() {
            eprintln!("skipping: {} not found", ls_path());
            return;
        }
    };
}

macro_rules! need_gnu {
    () => {
        need_ls!();
        if !system_ls_is_gnu() {
            eprintln!("skipping: {} is not GNU ls", ls_path());
            return;
        }
    };
}

#[test]
fn short_formats_match() {
    need_ls!();
    assert_same(
        &standard_tree(),
        &[
            &["-1"],
            &["-1a"],
            &["-1A"],
            &["-1F"],
            &["-1p"],
            &["-1r"],
            &["-1aF"],
            &["-1Ar"],
            &["-1i"],
            &["-1d"],
            &["-1d", "sub", "link-dir", "a.txt"],
            &["-1", "a.txt"],
            &["-1", "sub"],
            &["-1", "empty-dir"],
            &["-1", "link-dir"],
            &["-1d", "link-dir"],
            &["-1", "sub", "a.txt"],
            &["-1", "a.txt", "sub", "empty-dir"],
        ],
    );
}

#[test]
fn recursive_listings_match() {
    need_ls!();
    let f = standard_tree();
    // several operands: both print every heading
    assert_same(
        &f,
        &[&["-1R", "sub", "empty-dir"], &["-1Rp", "sub", "empty-dir"]],
    );
    let single: &[&[&str]] = &[&["-1R"], &["-1AR"], &["-1R", "sub"], &["-1Rp"]];
    if system_ls_is_gnu() {
        assert_same(&f, single);
        return;
    }
    // BSD ls leaves out the heading of a lone operand; the rest is the same
    for args in single {
        let mine = text(&eka(&f, args).stdout);
        let body = mine.split_once('\n').map_or("", |(_, rest)| rest);
        assert!(
            mine.lines().next().is_some_and(|l| l.ends_with(':')),
            "{args:?}"
        );
        assert_eq!(body, text(&sys_ls(&f, args).stdout), "{args:?}");
    }
}

#[test]
fn sort_orders_match() {
    need_ls!();
    assert_same(
        &flat_tree(),
        &[
            &["-1S"],
            &["-1Sr"],
            &["-1t"],
            &["-1tr"],
            &["-1ut"],
            &["-1r"],
        ],
    );
}

#[test]
fn missing_operands_fail_like_ls() {
    need_ls!();
    let f = standard_tree();
    for args in [&["nope"][..], &["-1", "nope", "a.txt"], &["-l", "nope"]] {
        let (mine, theirs) = (eka(&f, args), sys_ls(&f, args));
        assert!(
            !mine.status.success() && !theirs.status.success(),
            "{args:?}"
        );
        // what can be listed still is
        assert_eq!(mine.stdout, theirs.stdout, "{args:?}");
        assert!(
            !mine.stderr.is_empty() && !theirs.stderr.is_empty(),
            "{args:?}"
        );
    }
}

/// Mode without the BSD `@`/`+`/`.` suffix, then the remaining fields with
/// their padding collapsed. `total` lines are dropped (512 vs 1024 bytes).
fn normalize(out: &str, inode: bool) -> Vec<String> {
    out.lines()
        .filter(|l| !l.starts_with("total "))
        .map(|line| {
            let mut fields = line.split_whitespace();
            let first = if inode {
                fields.next().unwrap_or("").to_owned()
            } else {
                String::new()
            };
            let mode = fields
                .next()
                .unwrap_or("")
                .trim_end_matches(['@', '+', '.']);
            let rest: Vec<&str> = fields.collect();
            format!("{first} {mode} {}", rest.join(" "))
        })
        .collect()
}

fn assert_long_same(f: &Fixture, cases: &[&[&str]]) {
    let gnu = system_ls_is_gnu();
    let mut diffs = Vec::new();
    for args in cases {
        let (mine, theirs) = (eka(f, args), sys_ls(f, args));
        let inode = args
            .iter()
            .any(|a| a.starts_with('-') && !a.starts_with("--") && a.contains('i'));
        // gls does not print the macOS extended-attribute marker
        let same = if gnu && !cfg!(target_os = "macos") {
            mine.stdout == theirs.stdout
        } else {
            normalize(&text(&mine.stdout), inode) == normalize(&text(&theirs.stdout), inode)
        };
        if !same {
            diffs.push(format!(
                "{args:?}\n  eka:\n{}\n  ls:\n{}",
                text(&mine.stdout),
                text(&theirs.stdout)
            ));
        }
    }
    assert!(
        diffs.is_empty(),
        "differences from {}:\n{}",
        ls_path(),
        diffs.join("\n")
    );
}

#[test]
fn long_format_matches() {
    need_ls!();
    // Operands avoid symlinks, whose mtime is "now" and could differ by a minute
    let operands = [
        "a.txt",
        "a1",
        "a10",
        "a2",
        "backup~",
        "big.bin",
        "c.rs",
        "with space",
        "empty-dir",
        "sub",
    ];
    let with = |flags: &'static str| -> Vec<&'static str> {
        let mut v = vec![flags];
        v.extend(operands);
        v
    };
    let (l, ln, li, lr, lt) = (
        with("-l"),
        with("-ln"),
        with("-li"),
        with("-lr"),
        with("-lS"),
    );
    assert_long_same(
        &standard_tree(),
        &[
            &l,
            &ln,
            &li,
            &lr,
            &lt,
            &["-ld", "sub", "empty-dir"],
            &["-l", "sub"],
        ],
    );
    assert_long_same(
        &flat_tree(),
        &[
            &["-l"],
            &["-lt"],
            &["-lS"],
            &["-lu"],
            &["-lut"],
            &["-ln"],
            &["-lr"],
            &["-l", "f_big"],
        ],
    );
}

#[test]
fn long_format_with_inodes_and_classification_matches() {
    need_ls!();
    assert_long_same(&flat_tree(), &[&["-li"], &["-lF"], &["-liF"]]);
}

// ------------------------------------------------------------ GNU only

fn hostile_tree() -> Fixture {
    let f = Fixture::new();
    for name in [
        // A name with a newline is covered by its own test: how GNU ls shows
        // it unescaped depends on the coreutils version.
        "evil\u{1b}[2J",
        "tab\tname",
        "with space",
        "plain",
        "a$b",
        "x*y",
        "~tilde",
        "#hash",
        "eq=x",
        "back\\slash",
        "dq\"x",
        "semi;colon",
        "paren(s)",
    ] {
        f.file(name, 1, day(1));
    }
    f
}

#[test]
fn gnu_access_time_sort_matches() {
    need_gnu!();
    // BSD ls only uses the access time with -t or -l
    assert_same(&flat_tree(), &[&["-1u"], &["-1ur"], &["-1c"]]);
}

#[test]
fn gnu_sort_and_filter_options_match() {
    need_gnu!();
    assert_same(
        &standard_tree(),
        &[
            &["-1v"],
            &["-1vr"],
            &["-1X"],
            &["-1U"],
            &["-1", "--group-directories-first"],
            &["-1r", "--group-directories-first"],
            &["-1", "-I", "*.txt"],
            &["-1", "-I", "a?", "-I", "*.rs"],
            &["-1", "--hide=a*"],
            &["-1a", "--hide=a*"],
            &["-1A", "-I", ".*"],
            &["-1B"],
            &["-1", "--sort=size"],
            &["-1", "--sort=time"],
            &["-1", "--sort=version"],
            &["-1", "--sort=extension"],
            &["-1", "--sort=none"],
        ],
    );
}

#[test]
fn gnu_long_format_options_match() {
    need_gnu!();
    assert_same(
        &flat_tree(),
        &[
            &["-lh"],
            &["-lg"],
            &["-lo"],
            &["-lgo"],
            &["-lG"],
            &["-ls"],
            &["-lsh"],
            &["-lsi"],
            &["-l", "--block-size=K"],
            &["-l", "--block-size=100"],
            &["-s"],
            &["-1s"],
            &["-l", "--time-style=long-iso"],
            &["-l", "--time-style=iso"],
            &["-l", "--time-style=full-iso"],
            &["-l", "--time-style=+%Y/%m/%d"],
            &["-l", "--time-style=+new %H\nold %Y"],
            &["--full-time"],
            &["-l", "--time=atime"],
            &["-lt", "--time=atime"],
            &["-n"],
            &["-ln"],
            &["-lk"],
            &["-lA"],
        ],
    );
}

#[test]
fn gnu_layouts_match() {
    need_gnu!();
    let f = Fixture::new();
    for n in ["a", "bb", "ccc", "dd", "e", "ffff", "g", "hhhhh", "i", "jj"] {
        f.file(n, 1, day(1));
    }
    assert_same_columns(
        &f,
        &[
            &["-C", "-w", "20"],
            &["-C", "-w", "30"],
            &["-C", "-w", "80"],
            &["-C", "-w", "7"],
            &["-C", "-w", "1"],
            &["-x", "-w", "20"],
            &["-x", "-w", "30"],
            &["-x", "-w", "80"],
            &["-m", "-w", "20"],
            &["-m", "-w", "30"],
            &["-m", "-w", "80"],
            &["-C", "-w", "0"],
            &["-Cs", "-w", "30"],
            &["-Ci", "-w", "40"],
            &["-CF", "-w", "30"],
        ],
    );
}

#[test]
fn gnu_newline_in_a_name_is_escaped_like_gnu() {
    need_gnu!();
    let f = Fixture::new();
    f.file("line\nbreak", 1, day(1)).file("plain", 1, day(1));
    // Only the styles that escape it: with the others, GNU ls prints the
    // newline raw or as "?" depending on the coreutils version, and eka
    // always prints "?" so one entry cannot span two lines.
    assert_same(
        &f,
        &[
            &["-1", "--quoting-style=shell-escape"],
            &["-1", "--quoting-style=shell-escape-always"],
            &["-1", "--quoting-style=c"],
            &["-1", "--quoting-style=escape"],
            &["-1b"],
            &["-1Q"],
        ],
    );
}

#[test]
fn gnu_quoting_styles_match() {
    need_gnu!();
    let mut cases: Vec<Vec<&str>> = Vec::new();
    for style in [
        "literal",
        "shell",
        "shell-always",
        "shell-escape",
        "shell-escape-always",
        "c",
        "escape",
    ] {
        cases.push(vec![
            "-1",
            Box::leak(format!("--quoting-style={style}").into_boxed_str()),
        ]);
    }
    for flags in ["-1b", "-1Q", "-1N", "-1q"] {
        cases.push(vec![flags]);
    }
    cases.push(vec!["-m", "-w", "60", "--quoting-style=shell-escape"]);
    cases.push(vec!["-l", "--quoting-style=shell-escape"]);
    let refs: Vec<&[&str]> = cases.iter().map(Vec::as_slice).collect();
    assert_same(&hostile_tree(), &refs);
    assert_same_columns(
        &hostile_tree(),
        &[&["-C", "-w", "60", "--quoting-style=shell-escape"]],
    );
}

#[test]
fn gnu_quoting_of_symlink_targets_and_headings_matches() {
    need_gnu!();
    let f = Fixture::new();
    f.dir("d e")
        .file("d e/f g", 1, day(1))
        .link("t\u{1b}t", "ln");
    assert_same(
        &f,
        &[
            &["-R", "--quoting-style=shell-escape"],
            &["-R", "-Q"],
            &["-R", "-b"],
            &["-1", "d e", "--quoting-style=shell-escape"],
        ],
    );
}

#[test]
fn device_numbers_match_ls() {
    // BSD ls prints the device number in hex, so only GNU is a reference
    if !has_system_ls() || !system_ls_is_gnu() {
        return;
    }
    let f = hostile_tree();
    // Fields: mode links owner group major, minor ...
    let numbers = |out: &std::process::Output| {
        let line = text(&out.stdout);
        let fields: Vec<&str> = line.split_whitespace().collect();
        (fields[4].to_string(), fields[5].to_string())
    };
    for dev in ["/dev/null", "/dev/zero"] {
        let args = ["-l", dev];
        assert_eq!(
            numbers(&eka(&f, &args)),
            numbers(&sys_ls(&f, &args)),
            "{dev}"
        );
    }
}

#[test]
fn gnu_format_and_indicator_options_match() {
    need_ls!();
    if !system_ls_is_gnu() {
        return;
    }
    assert_same(
        &standard_tree(),
        &[
            &["--format=single-column"],
            &["--format=commas", "-w", "40"],
            &["--format=across", "-w", "40"],
            &["--format=horizontal", "-w", "40"],
            &["--format=vertical", "-w", "40"],
            &["--file-type", "-1"],
            &["--indicator-style=file-type", "-1"],
            &["--indicator-style=slash", "-1"],
            &["--indicator-style=none", "-1"],
        ],
    );
    assert_long_same(
        &standard_tree(),
        &[
            &["--format=long", "a.txt", "big.bin"],
            &["-l", "--si", "a.txt", "big.bin"],
            &["-lh", "--si", "big.bin"],
            &["-ls", "--si", "big.bin"],
        ],
    );
}

/// Runs `program` in the fixture under a UTF-8 locale
fn in_locale(program: &str, f: &Fixture, args: &[&str]) -> String {
    let mut cmd = std::process::Command::new(program);
    cmd.current_dir(f.root())
        .args(args)
        .env_clear()
        .env("LC_ALL", "en_US.UTF-8")
        .env("TZ", "UTC");
    text(&cmd.output().unwrap().stdout)
}

#[test]
fn gnu_locale_sort_order_matches() {
    need_ls!();
    if !system_ls_is_gnu() {
        return;
    }
    let f = Fixture::new();
    for name in [
        "a", "B", "c", "é", "e", "ñ", "n", "_x", ".h", "1", "10", "2", "a_b", "a-b", "a.b", "ab",
        "ab~", "Zed", "zed.txt", "b.tar.gz",
    ] {
        f.file(name, 1, day(1));
    }
    // Without the locale installed GNU ls sorts bytewise: nothing to compare
    if in_locale(&ls_path(), &f, &["-1A"]) == text(&sys_ls(&f, &["-1A"]).stdout) {
        return;
    }
    for args in [&["-1A"][..], &["-1Ar"], &["-1AX"], &["-1AS"], &["-1At"]] {
        assert_eq!(
            in_locale(EKA, &f, args),
            in_locale(&ls_path(), &f, args),
            "{args:?}"
        );
    }
}

#[test]
fn gnu_last_option_wins() {
    need_ls!();
    if !system_ls_is_gnu() {
        return;
    }
    assert_same(
        &standard_tree(),
        &[
            &["-vX"],
            &["-Xv"],
            &["-tS"],
            &["-St"],
            &["-Ut"],
            &["-tU"],
            &["-fS"],
            &["-Sf"],
            &["-1", "-C", "-w", "40"],
            &["-C", "-1", "-w", "40"],
            &["-x", "-C", "-w", "40"],
            &["-m", "-C", "-w", "40"],
            &["-C", "-m", "-w", "40"],
            &["-F", "-p"],
            &["-p", "-F"],
            &["-F", "--file-type"],
            &["--file-type", "-F"],
            &["--indicator-style=none", "-F"],
            &["-F", "--indicator-style=none"],
            &["-a", "-A"],
            &["-A", "-a"],
            &["-aa"],
            &["-1", "-1"],
            &["-Q", "-b"],
            &["-b", "-Q"],
            &["-N", "-Q"],
            &["-Q", "-N"],
            &["-u", "-c"],
            &["-c", "-u"],
            &["-t", "-u"],
        ],
    );
    assert_long_same(
        &standard_tree(),
        &[
            &["-l", "-1", "a.txt", "big.bin"],
            &["-1", "-l", "a.txt", "big.bin"],
            &["-l", "-m", "-w", "80", "a.txt", "big.bin"],
            &["-m", "-l", "a.txt", "big.bin"],
            &["-n", "-1", "a.txt", "big.bin"],
            &["-l", "-h", "-k", "big.bin"],
            &["-l", "-k", "-h", "big.bin"],
            &["-l", "-h", "--si", "big.bin"],
            &["-l", "--si", "-h", "big.bin"],
            &["-l", "-h", "--block-size=1", "big.bin"],
            &["-l", "--block-size=1", "-h", "big.bin"],
            &["-lu", "-c", "a.txt", "big.bin"],
            &["-lc", "-u", "a.txt", "big.bin"],
        ],
    );
}
