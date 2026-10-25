//! End-to-end behavior of the eka binary. Expected outputs are written by hand
//! from the fixtures and from GNU ls semantics, not copied from eka's output.

mod common;

use common::*;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

/// Visible entries of `standard_tree()` in byte order
const VISIBLE: [&str; 13] = [
    "a.txt",
    "a1",
    "a10",
    "a2",
    "backup~",
    "big.bin",
    "c.rs",
    "empty-dir",
    "link-broken",
    "link-dir",
    "link-ok",
    "sub",
    "with space",
];

fn with(prefix: &[&str], rest: &[&str]) -> Vec<String> {
    strs(&[prefix, rest].concat())
}

// ---------------------------------------------------------------- listing

#[test]
fn default_listing_is_sorted_bytewise_without_hidden() {
    assert_eq!(names(&standard_tree(), &[]), strs(&VISIBLE));
}

#[test]
fn all_adds_dot_entries_and_almost_all_does_not() {
    let f = standard_tree();
    assert_eq!(
        names(&f, &["-a"]),
        with(&[".", "..", ".hdir", ".hidden"], &VISIBLE)
    );
    assert_eq!(names(&f, &["-A"]), with(&[".hdir", ".hidden"], &VISIBLE));
}

#[test]
fn reverse_reverses_the_whole_order() {
    let mut expected = strs(&VISIBLE);
    expected.reverse();
    assert_eq!(names(&standard_tree(), &["-r"]), expected);
}

#[test]
fn classify_appends_indicators() {
    let expected: Vec<String> = VISIBLE
        .iter()
        .map(|n| match *n {
            "big.bin" => "big.bin*".into(),
            "empty-dir" | "sub" => format!("{n}/"),
            "link-broken" | "link-dir" | "link-ok" => format!("{n}@"),
            other => other.into(),
        })
        .collect();
    assert_eq!(names(&standard_tree(), &["-F"]), expected);
}

#[test]
fn slash_indicator_marks_only_directories() {
    let expected: Vec<String> = VISIBLE
        .iter()
        .map(|n| match *n {
            "empty-dir" | "sub" => format!("{n}/"),
            other => other.into(),
        })
        .collect();
    assert_eq!(names(&standard_tree(), &["-p"]), expected);
    // -F wins over -p
    assert_eq!(
        names(&standard_tree(), &["-p", "-F"]),
        names(&standard_tree(), &["-F"])
    );
}

#[test]
fn directory_flag_lists_operands_themselves_sorted() {
    let f = standard_tree();
    assert_eq!(
        names(&f, &["-d", "sub", "link-dir", "a.txt"]),
        strs(&["a.txt", "link-dir", "sub"])
    );
    assert_eq!(names(&f, &["-d"]), strs(&["."]));
}

#[test]
fn files_are_listed_before_directories_with_headings() {
    let f = standard_tree();
    assert_eq!(
        ok(&eka(&f, &["-1", "sub", "a.txt"])),
        "a.txt\n\nsub:\ninner.txt\nsub2\n"
    );
    assert_eq!(
        ok(&eka(&f, &["-1", "sub", "empty-dir"])),
        "empty-dir:\n\nsub:\ninner.txt\nsub2\n"
    );
}

#[test]
fn directory_operands_are_sorted_with_the_same_options() {
    let f = standard_tree();
    let heads = |args: &[&str]| -> Vec<String> {
        ok(&eka(&f, args))
            .lines()
            .filter(|l| l.ends_with(':'))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(heads(&["-1", "sub", "empty-dir"]), ["empty-dir:", "sub:"]);
    assert_eq!(heads(&["-1r", "empty-dir", "sub"]), ["sub:", "empty-dir:"]);
    // by time: empty-dir and sub share a timestamp, so the name decides; -t on
    // directories with different times orders them newest first
    let g = Fixture::new();
    g.dir("old")
        .dir("new")
        .dir_time("old", day(1))
        .dir_time("new", day(9));
    let out = ok(&eka(&g, &["-1t", "old", "new"]));
    assert_eq!(out, "new:\n\nold:\n");
}

#[test]
fn symlink_to_directory_operand_is_followed_unless_long_or_directory() {
    let f = standard_tree();
    assert_eq!(names(&f, &["link-dir"]), strs(&["inner.txt", "sub2"]));
    assert_eq!(names(&f, &["-d", "link-dir"]), strs(&["link-dir"]));
    assert_eq!(names(&f, &["link-dir/"]), strs(&["inner.txt", "sub2"]));
}

#[test]
fn size_and_time_sorts() {
    let f = flat_tree();
    assert_eq!(
        names(&f, &["-S"]),
        strs(&["f_big", "f_mid", "f_mid2", "f_small"])
    );
    assert_eq!(
        names(&f, &["-t"]),
        strs(&["f_small", "f_big", "f_mid2", "f_mid"])
    );
    assert_eq!(
        names(&f, &["-tr"]),
        strs(&["f_mid", "f_mid2", "f_big", "f_small"])
    );
    assert_eq!(
        names(&f, &["--sort=size", "-r"]),
        strs(&["f_small", "f_mid2", "f_mid", "f_big"])
    );
}

#[test]
fn access_time_sort_and_display() {
    let f = flat_tree();
    // atimes: f_mid > f_small > f_big > f_mid2
    assert_eq!(
        names(&f, &["-u"]),
        strs(&["f_mid", "f_small", "f_big", "f_mid2"])
    );
    // with -l, -u only changes the displayed time: sort stays by name
    let out = ok(&eka(&f, &["-lu"]));
    let order: Vec<&str> = out
        .lines()
        .skip(1) // the total line
        .map(|l| l.rsplit(' ').next().unwrap())
        .collect();
    assert_eq!(order, ["f_big", "f_mid", "f_mid2", "f_small"]);
    assert!(
        out.lines().nth(1).unwrap().contains("Feb 20  2020"),
        "{out}"
    );
}

#[test]
fn change_time_is_accepted() {
    let f = flat_tree();
    assert_eq!(names(&f, &["-c"]).len(), 4);
    assert_eq!(names(&f, &["--time=ctime", "-t"]).len(), 4);
}

#[test]
fn version_sort() {
    let f = Fixture::new();
    for n in ["f10", "f2", "f1", "f01", "g1"] {
        f.file(n, 1, day(1));
    }
    assert_eq!(names(&f, &["-v"]), strs(&["f01", "f1", "f2", "f10", "g1"]));
    assert_eq!(
        names(&f, &["--sort=version"]),
        strs(&["f01", "f1", "f2", "f10", "g1"])
    );
    // plain name order puts f10 before f2
    assert_eq!(names(&f, &[]), strs(&["f01", "f1", "f10", "f2", "g1"]));
    assert_eq!(names(&f, &["-vr"]), strs(&["g1", "f10", "f2", "f1", "f01"]));
}

#[test]
fn extension_sort() {
    let f = Fixture::new();
    for n in ["x.rs", "y.txt", "z", "a.rs", "b.txt"] {
        f.file(n, 1, day(1));
    }
    let expected = strs(&["z", "a.rs", "x.rs", "b.txt", "y.txt"]);
    assert_eq!(names(&f, &["-X"]), expected);
    assert_eq!(names(&f, &["--sort=extension"]), expected);
}

#[test]
fn unsorted_lists_the_same_entries() {
    let f = standard_tree();
    let mut got = names(&f, &["-U"]);
    got.sort();
    assert_eq!(got, strs(&VISIBLE));
    let mut got = names(&f, &["--sort=none"]);
    got.sort();
    assert_eq!(got, strs(&VISIBLE));
}

#[test]
fn sort_option_precedence() {
    let f = flat_tree();
    // --sort beats -S; -S beats -t
    assert_eq!(
        names(&f, &["-S", "--sort=time"]),
        strs(&["f_small", "f_big", "f_mid2", "f_mid"])
    );
    assert_eq!(
        names(&f, &["-t", "-S"]),
        strs(&["f_big", "f_mid", "f_mid2", "f_small"])
    );
}

#[test]
fn directories_can_be_grouped_first() {
    let f = standard_tree();
    // a symlink to a directory is grouped with the directories, as in GNU ls
    let rest: Vec<&str> = VISIBLE
        .iter()
        .copied()
        .filter(|n| !["empty-dir", "link-dir", "sub"].contains(n))
        .collect();
    assert_eq!(
        names(&f, &["--group-directories-first"]),
        with(&["empty-dir", "link-dir", "sub"], &rest)
    );
    // -r reverses inside each group but directories stay first
    let mut reversed = rest.clone();
    reversed.reverse();
    assert_eq!(
        names(&f, &["--group-directories-first", "-r"]),
        with(&["sub", "link-dir", "empty-dir"], &reversed)
    );
}

// ---------------------------------------------------------------- filters

fn without(drop: &[&str]) -> Vec<String> {
    VISIBLE
        .iter()
        .filter(|n| !drop.contains(n))
        .map(|n| (*n).to_owned())
        .collect()
}

#[test]
fn ignore_patterns() {
    let f = standard_tree();
    assert_eq!(names(&f, &["-I", "*.txt"]), without(&["a.txt"]));
    assert_eq!(
        names(&f, &["-I", "*.txt", "-I", "a?"]),
        without(&["a.txt", "a1", "a2"])
    );
    assert_eq!(
        names(&f, &["--ignore=a[12]*"]),
        without(&["a1", "a10", "a2"])
    );
    assert_eq!(names(&f, &["-I", "a[!1]*"]), without(&["a.txt", "a2"]));
    // an unterminated class is a literal bracket and matches nothing here
    assert_eq!(names(&f, &["-I", "[abc"]), strs(&VISIBLE));
}

#[test]
fn hide_is_overridden_by_all_but_ignore_is_not() {
    let f = standard_tree();
    assert_eq!(
        names(&f, &["--hide=a*"]),
        without(&["a.txt", "a1", "a10", "a2"])
    );
    assert_eq!(
        names(&f, &["-A", "--hide=a*"]),
        with(&[".hdir", ".hidden"], &VISIBLE)
    );
    assert_eq!(
        names(&f, &["-a", "--hide=a*"]),
        with(&[".", "..", ".hdir", ".hidden"], &VISIBLE)
    );
    // -I still applies with -a; ".*" also matches "." and ".."
    assert_eq!(names(&f, &["-a", "-I", ".*"]), strs(&VISIBLE));
}

#[test]
fn ignore_backups() {
    assert_eq!(names(&standard_tree(), &["-B"]), without(&["backup~"]));
}

#[test]
fn filters_never_apply_to_operands() {
    let f = standard_tree();
    assert_eq!(names(&f, &["-I", "*", "a.txt"]), strs(&["a.txt"]));
    assert_eq!(names(&f, &["-B", "backup~"]), strs(&["backup~"]));
}

// ------------------------------------------------------------ long format

fn long_lines(f: &Fixture, args: &[&str]) -> Vec<String> {
    let mut all = vec!["-l"];
    all.extend_from_slice(args);
    ok(&eka(f, &all)).lines().map(str::to_owned).collect()
}

#[test]
fn long_format_layout_and_columns() {
    let f = flat_tree();
    let (user, group) = user_and_group();
    let out = ok(&eka(&f, &["-l"]));
    let mut lines = out.lines();
    let total = lines.next().unwrap();
    assert!(
        total
            .strip_prefix("total ")
            .is_some_and(|n| n.parse::<u64>().is_ok()),
        "{total}"
    );
    let rows: Vec<&str> = lines.collect();
    assert_eq!(
        rows,
        [
            format!("-rw-r--r-- 1 {user} {group} 500 Mar  1  2020 f_big"),
            format!("-rw-r--r-- 1 {user} {group}  50 Jan  1  2020 f_mid"),
            format!("-rw-r--r-- 1 {user} {group}  50 Feb  1  2020 f_mid2"),
            format!("-rw-r--r-- 1 {user} {group}   5 May  1  2020 f_small"),
        ]
    );
}

#[test]
fn numeric_ids_and_omitted_columns() {
    let f = flat_tree();
    let (user, group) = user_and_group();
    let id = |flag: &str| {
        let out = std::process::Command::new("id").arg(flag).output().unwrap();
        text(&out.stdout).trim().to_owned()
    };
    let (uid, gid) = (id("-u"), id("-g"));
    let first = |args: &[&str]| long_lines(&f, args)[1].clone();

    assert_eq!(
        first(&["-n"]),
        format!("-rw-r--r-- 1 {uid} {gid} 500 Mar  1  2020 f_big")
    );
    assert_eq!(
        first(&["-g"]),
        format!("-rw-r--r-- 1 {group} 500 Mar  1  2020 f_big")
    );
    assert_eq!(
        first(&["-o"]),
        format!("-rw-r--r-- 1 {user} 500 Mar  1  2020 f_big")
    );
    assert_eq!(
        first(&["-G"]),
        format!("-rw-r--r-- 1 {user} 500 Mar  1  2020 f_big")
    );
    assert_eq!(first(&["-go"]), "-rw-r--r-- 1 500 Mar  1  2020 f_big");
    // -n, -g and -o imply -l by themselves
    let implied = ok(&eka(&f, &["-g"]));
    assert!(implied.starts_with("total "), "{implied}");
    // -G alone does not
    assert_eq!(names(&f, &["-G"]).len(), 4);
}

#[test]
fn human_readable_sizes_round_up() {
    let f = Fixture::new();
    for (name, len) in [
        ("s0", 0),
        ("s1k", 1024),
        ("s15", 1536),
        ("s10k", 10 * 1024),
        ("s1m", 1024 * 1024),
        ("s1025", 1025),
    ] {
        f.file(name, len, day(1));
    }
    let sizes: Vec<(String, String)> = long_lines(&f, &["-h"])
        .iter()
        .skip(1)
        .map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            (t[8].to_owned(), t[4].to_owned())
        })
        .collect();
    let got: Vec<(&str, &str)> = sizes
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("s0", "0"),
            ("s1025", "1.1K"),
            ("s10k", "10K"),
            ("s15", "1.5K"),
            ("s1k", "1.0K"),
            ("s1m", "1.0M"),
        ]
    );
}

#[test]
fn allocated_size_column_goes_before_the_mode() {
    let f = flat_tree();
    let lines = long_lines(&f, &["-s"]);
    for row in &lines[1..] {
        let t: Vec<&str> = row.split_whitespace().collect();
        assert!(t[0].parse::<u64>().is_ok(), "{row}");
        assert_eq!(t[1], "-rw-r--r--", "{row}");
    }
    // short format: size then name
    let short = names(&f, &["-s"]);
    assert_eq!(short.len(), 5, "a total line plus four entries: {short:?}");
    assert!(short[0].starts_with("total "));
    for row in &short[1..] {
        let (size, name) = row.split_once(' ').unwrap();
        assert!(
            size.parse::<u64>().is_ok() && name.starts_with("f_"),
            "{row}"
        );
    }
}

#[test]
fn block_size_scales_the_size_column() {
    let f = flat_tree();
    let sizes = |args: &[&str]| -> Vec<String> {
        long_lines(&f, args)
            .iter()
            .skip(1)
            .map(|l| l.split_whitespace().nth(4).unwrap().to_owned())
            .collect()
    };
    assert_eq!(sizes(&["--block-size=100"]), ["5", "1", "1", "1"]);
    // a bare unit prints its suffix; with a number in front it does not
    assert_eq!(sizes(&["--block-size=K"]), ["1K", "1K", "1K", "1K"]);
    assert_eq!(sizes(&["--block-size=KB"]), ["1kB", "1kB", "1kB", "1kB"]);
    assert_eq!(
        sizes(&["--block-size=KiB"]),
        ["1KiB", "1KiB", "1KiB", "1KiB"]
    );
    assert_eq!(sizes(&["--block-size=1K"]), ["1", "1", "1", "1"]);
    assert_eq!(sizes(&["--block-size=2K"]), ["1", "1", "1", "1"]);
    assert_eq!(sizes(&["--block-size=1KB"]), ["1", "1", "1", "1"]);
    assert_eq!(sizes(&["--block-size=1"]), ["500", "50", "50", "5"]);
    let total = long_lines(&f, &["--block-size=K"])[0].clone();
    assert!(
        total.starts_with("total ") && total.ends_with('K'),
        "{total}"
    );
    let total = long_lines(&f, &["--block-size=1024"])[0].clone();
    assert!(
        total.trim_end_matches(|c: char| c.is_ascii_digit()) == "total ",
        "{total}"
    );
    // -h overrides --block-size
    assert_eq!(sizes(&["-h", "--block-size=100"]), ["500", "50", "50", "5"]);
}

#[test]
fn time_styles() {
    let f = flat_tree();
    let first = |args: &[&str]| long_lines(&f, args)[1].clone();
    let tail = |row: &str, n: usize| {
        let t: Vec<&str> = row.split_whitespace().collect();
        t[t.len() - n..].join(" ")
    };
    assert_eq!(
        tail(&first(&["--time-style=long-iso"]), 3),
        "2020-03-01 12:00 f_big"
    );
    assert!(
        first(&["--time-style=full-iso"]).contains("2020-03-01 12:00:00.000000000 +0000 f_big")
    );
    // old files show only the date in iso style; the column keeps its width
    assert!(first(&["--time-style=iso"]).ends_with(" 2020-03-01  f_big"));
    assert!(first(&["--time-style=+%Y/%m"]).ends_with(" 2020/03 f_big"));
    assert!(first(&["--time-style=locale"]).ends_with(" Mar  1  2020 f_big"));
    // a newline splits the old format from the recent one
    // (GNU: the first format is for older files, the second for recent ones)
    assert!(first(&["--time-style=+old %Y\nnew %H:%M"]).ends_with(" old 2020 f_big"));
    let full = ok(&eka(&f, &["--full-time"]));
    assert!(
        full.contains("2020-03-01 12:00:00.000000000 +0000 f_big"),
        "{full}"
    );
    assert!(full.starts_with("total "), "--full-time implies -l");
}

#[test]
fn time_style_environment_variable() {
    let f = flat_tree();
    let long = |env: &[(&str, &str)], args: &[&str]| {
        ok(&eka_env(&f, args, env))
            .lines()
            .nth(1)
            .unwrap()
            .to_owned()
    };
    assert!(long(&[("TIME_STYLE", "long-iso")], &["-l"]).contains(" 2020-03-01 12:00 f_big"));
    // an explicit option wins
    assert!(
        long(&[("TIME_STYLE", "long-iso")], &["-l", "--time-style=iso"])
            .ends_with(" 2020-03-01  f_big")
    );
    // an invalid value is ignored instead of failing
    assert!(long(&[("TIME_STYLE", "+%Q")], &["-l"]).ends_with(" Mar  1  2020 f_big"));
    assert!(long(&[("TIME_STYLE", "bogus")], &["-l"]).ends_with(" Mar  1  2020 f_big"));
}

#[test]
fn recent_files_show_the_time_of_day() {
    let f = Fixture::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    f.file("fresh", 1, now - 86_400);
    f.file("stale", 1, now - 400 * 86_400);
    let rows = long_lines(&f, &[]);
    let date = |row: &str| {
        row.split_whitespace()
            .skip(5)
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (fresh, stale) = (date(&rows[1]), date(&rows[2]));
    assert!(fresh.contains(':'), "{fresh}");
    assert!(
        !stale.contains(':') && stale.ends_with(&format!("{}", year_of(now - 400 * 86_400))),
        "{stale}"
    );
}

#[test]
fn the_recent_window_is_about_six_months() {
    let f = Fixture::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    // well inside and well outside half a year, away from the boundary
    f.file("d090", 1, now - 90 * 86_400);
    f.file("d150", 1, now - 150 * 86_400);
    f.file("d220", 1, now - 220 * 86_400);
    f.file("d300", 1, now - 300 * 86_400);
    let rows = long_lines(&f, &[]);
    let shows_time = |name: &str| {
        let row = rows.iter().find(|r| r.ends_with(name)).unwrap();
        row.split_whitespace()
            .nth(7)
            .is_some_and(|t| t.contains(':'))
    };
    assert!(shows_time("d090") && shows_time("d150"), "{rows:?}");
    assert!(!shows_time("d220") && !shows_time("d300"), "{rows:?}");
}

fn year_of(secs: i64) -> i64 {
    // civil-from-days (Howard Hinnant), enough for a year number
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    yoe + era * 400 + i64::from(month <= 2)
}

// ------------------------------------------------------------- symlinks

fn links_tree() -> Fixture {
    let f = Fixture::new();
    f.file("target.txt", 10, day(1))
        .dir("dd")
        .file("dd/x", 1, day(2))
        .link("target.txt", "ok")
        .link("missing", "broken")
        .link("dd", "dirlink");
    f
}

#[test]
fn long_format_shows_link_targets() {
    let f = links_tree();
    let rows = long_lines(&f, &[]);
    let ends = |suffix: &str| {
        rows.iter()
            .any(|r| r.starts_with('l') && r.ends_with(suffix))
    };
    assert!(ends(" broken -> missing"), "{rows:?}");
    assert!(ends(" dirlink -> dd"), "{rows:?}");
    assert!(ends(" ok -> target.txt"), "{rows:?}");
    // the size of a link is the length of its target path
    let ok_row = rows
        .iter()
        .find(|r| r.ends_with(" ok -> target.txt"))
        .unwrap();
    assert_eq!(ok_row.split_whitespace().nth(4), Some("10"));
}

#[test]
fn dereference_shows_the_target_but_keeps_broken_links() {
    let f = links_tree();
    let rows = long_lines(&f, &["-L"]);
    let row = |name: &str| rows.iter().find(|r| r.ends_with(name)).unwrap().clone();
    assert!(row(" ok").starts_with("-rw-r--r-- 1"), "{rows:?}");
    assert!(!row(" ok").contains("->"));
    assert_eq!(row(" ok").split_whitespace().nth(4), Some("10"));
    assert!(row(" dirlink").starts_with('d'), "{rows:?}");
    assert!(row(" broken -> missing").starts_with('l'), "{rows:?}");
}

#[test]
fn command_line_dereference_flags() {
    let f = links_tree();
    // -l shows the link itself; -H follows it
    let plain = ok(&eka(&f, &["-l", "dirlink"]));
    assert!(
        plain.contains("dirlink -> dd") && plain.starts_with('l'),
        "{plain}"
    );
    let followed = ok(&eka(&f, &["-lH", "dirlink"]));
    assert!(
        followed.contains(" x") && !followed.contains("->"),
        "{followed}"
    );
    // -H does not dereference links found inside a directory
    let inside = long_lines(&f, &["-H"]);
    assert!(inside.iter().any(|r| r.ends_with(" ok -> target.txt")));
    // -P overrides -H and -L
    let p = ok(&eka(&f, &["-lLP", "dirlink"]));
    assert!(p.contains("dirlink -> dd"), "{p}");
}

// -------------------------------------------------------------- layout

fn cols_tree() -> Fixture {
    let f = Fixture::new();
    for n in ["a", "bb", "ccc", "dd", "e"] {
        f.file(n, 1, day(1));
    }
    f
}

#[test]
fn columns_fill_down_and_have_no_trailing_space() {
    let f = cols_tree();
    assert_eq!(
        ok(&eka(&f, &["-C", "-T", "0", "-w", "20"])),
        "a  bb  ccc  dd  e\n"
    );
    assert_eq!(
        ok(&eka(&f, &["-C", "-T", "0", "-w", "10"])),
        "a   ccc  e\nbb  dd\n"
    );
    assert_eq!(
        ok(&eka(&f, &["-C", "-T", "0", "-w", "0"])),
        "a  bb  ccc  dd  e\n"
    );
    // too narrow: one column
    assert_eq!(
        ok(&eka(&f, &["-C", "-T", "0", "-w", "3"])),
        "a\nbb\nccc\ndd\ne\n"
    );
}

#[test]
fn across_fills_rows_first() {
    let f = cols_tree();
    assert_eq!(
        ok(&eka(&f, &["-x", "-T", "0", "-w", "10"])),
        "a    bb\nccc  dd\ne\n"
    );
    assert_eq!(
        ok(&eka(&f, &["-x", "-T", "0", "-w", "40"])),
        "a  bb  ccc  dd  e\n"
    );
}

#[test]
fn comma_list_wraps_at_the_width() {
    let f = cols_tree();
    assert_eq!(ok(&eka(&f, &["-m", "-w", "12"])), "a, bb, ccc,\ndd, e\n");
    assert_eq!(ok(&eka(&f, &["-m", "-w", "0"])), "a, bb, ccc, dd, e\n");
}

#[test]
fn width_comes_from_columns_when_not_given() {
    let f = cols_tree();
    let out = ok(&eka_env(&f, &["-C", "-T", "0"], &[("COLUMNS", "10")]));
    assert_eq!(out, "a   ccc  e\nbb  dd\n");
    // -w wins over $COLUMNS
    let out = ok(&eka_env(
        &f,
        &["-C", "-T", "0", "-w", "20"],
        &[("COLUMNS", "10")],
    ));
    assert_eq!(out, "a  bb  ccc  dd  e\n");
}

#[test]
fn not_a_terminal_means_one_per_line() {
    assert_eq!(ok(&eka(&cols_tree(), &[])), "a\nbb\nccc\ndd\ne\n");
}

#[test]
fn columns_are_padded_with_tabs_by_default_like_gnu_ls() {
    let f = cols_tree();
    // "dd" ends at column 14 and the next column starts at 16: one tab stop
    assert_eq!(ok(&eka(&f, &["-C", "-w", "20"])), "a  bb  ccc  dd\te\n");
    assert_eq!(
        ok(&eka(&f, &["-C", "-w", "20", "-T", "8"])),
        "a  bb  ccc  dd\te\n"
    );
    assert_eq!(
        ok(&eka(&f, &["-C", "-w", "20", "-T", "0"])),
        "a  bb  ccc  dd  e\n"
    );
    assert_eq!(
        ok(&eka(&f, &["-x", "-w", "20", "--tabsize=0"])),
        "a  bb  ccc  dd  e\n"
    );
}

// ------------------------------------------------------- quoting / safety

fn evil_tree() -> Fixture {
    let f = Fixture::new();
    for name in [
        "evil\u{1b}[2J",
        "it's",
        "line\nbreak",
        "rtl\u{202e}txt",
        "tab\tname",
    ] {
        f.file(name, 1, day(1));
    }
    f
}

fn one(f: &Fixture, args: &[&str]) -> String {
    ok(&eka(f, args))
}

#[test]
fn literal_output_is_raw_when_not_a_terminal() {
    // like ls: a pipe gets the bytes as they are, except that a newline
    // becomes "?" so one entry cannot span two lines
    let out = eka(&evil_tree(), &["-1"]);
    assert_eq!(
        text(&out.stdout),
        "evil\u{1b}[2J\nit's\nline?break\nrtl\u{202e}txt\ntab\tname\n"
    );
}

#[test]
fn escape_style() {
    let expected = "evil\\033[2J\nit's\nline\\nbreak\nrtl\\342\\200\\256txt\ntab\\tname\n";
    assert_eq!(one(&evil_tree(), &["-1", "-b"]), expected);
    assert_eq!(one(&evil_tree(), &["-1", "--escape"]), expected);
    assert_eq!(
        one(&evil_tree(), &["-1", "--quoting-style=escape"]),
        expected
    );
}

#[test]
fn quote_name_style() {
    let expected =
        "\"evil\\033[2J\"\n\"it's\"\n\"line\\nbreak\"\n\"rtl\\342\\200\\256txt\"\n\"tab\\tname\"\n";
    assert_eq!(one(&evil_tree(), &["-1", "-Q"]), expected);
    assert_eq!(one(&evil_tree(), &["-1", "--quoting-style=c"]), expected);
}

#[test]
fn hide_control_chars_replaces_them_with_question_marks() {
    assert_eq!(
        one(&evil_tree(), &["-1", "-q"]),
        "evil?[2J\nit's\nline?break\nrtl?txt\ntab?name\n"
    );
}

#[test]
fn shell_escape_style() {
    assert_eq!(
        one(&evil_tree(), &["-1", "--quoting-style=shell-escape"]),
        "'evil'$'\\033''[2J'\n'it'\\''s'\n'line'$'\\n''break'\n'rtl'$'\\342\\200\\256''txt'\n'tab'$'\\t''name'\n"
    );
}

#[test]
fn shell_style_quotes_only_when_needed() {
    let f = Fixture::new();
    f.file("plain", 1, day(1)).file("with space", 1, day(1));
    assert_eq!(
        one(&f, &["-1", "--quoting-style=shell"]),
        "plain\n'with space'\n"
    );
    assert_eq!(
        one(&f, &["-1", "--quoting-style=shell-always"]),
        "'plain'\n'with space'\n"
    );
    // -N turns quoting off
    assert_eq!(one(&f, &["-1", "-N"]), "plain\nwith space\n");
}

#[test]
fn unquoted_names_get_a_leading_space_in_column_and_long_layouts() {
    let f = Fixture::new();
    f.file("plain", 1, day(1)).file("with space", 1, day(1));
    // columns line the names up
    assert_eq!(
        one(&f, &["-C", "-w", "80", "-T", "0", "--quoting-style=shell"]),
        " plain  'with space'\n"
    );
    // one per line and comma layouts do not pad
    assert_eq!(
        one(&f, &["-m", "-w", "80", "--quoting-style=shell"]),
        "plain, 'with space'\n"
    );
    let long = one(&f, &["-l", "--quoting-style=shell"]);
    // the long layout does: the usual separator plus one extra space
    assert!(
        long.lines().nth(1).unwrap().ends_with("2020  plain"),
        "{long}"
    );
    assert!(
        long.lines().nth(2).unwrap().ends_with("2020 'with space'"),
        "{long}"
    );
}

#[test]
fn quoting_style_environment_variable() {
    let f = evil_tree();
    let out = ok(&eka_env(&f, &["-1"], &[("QUOTING_STYLE", "escape")]));
    assert!(out.contains("evil\\033[2J"), "{out}");
    // an option wins over the environment, an invalid value is ignored
    let out = ok(&eka_env(&f, &["-1", "-Q"], &[("QUOTING_STYLE", "escape")]));
    assert!(out.starts_with('"'), "{out}");
    let out = ok(&eka_env(&f, &["-1"], &[("QUOTING_STYLE", "bogus")]));
    assert!(out.contains('\u{1b}'));
}

#[test]
fn safe_styles_emit_no_control_bytes() {
    let f = evil_tree();
    for args in [
        &["-1", "-b"][..],
        &["-1", "-Q"],
        &["-1", "-q"],
        &["-1", "--quoting-style=shell-escape"],
        &["-1", "--quoting-style=shell-escape-always"],
        &["-C", "-T", "0", "-w", "80", "-b"],
        &["-m", "-w", "80", "-Q"],
        &["-l", "-b"],
        &["-l", "--quoting-style=shell-escape"],
    ] {
        let out = eka(&f, args).stdout;
        let bad = out.iter().any(|b| (*b < 0x20 && *b != b'\n') || *b == 0x7f);
        assert!(!bad, "{args:?}: {:?}", text(&out));
        assert!(!text(&out).contains('\u{202e}'), "{args:?}");
        // every name stayed on its own line: nothing injected a line break
        if args.contains(&"-1") {
            assert_eq!(out.iter().filter(|b| **b == b'\n').count(), 5, "{args:?}");
        }
    }
}

#[test]
fn long_format_quotes_names_and_link_targets() {
    let f = Fixture::new();
    f.link("t\u{1b}t", "ln");
    let out = one(&f, &["-l", "--quoting-style=escape"]);
    assert!(out.trim_end().ends_with(" ln -> t\\033t"), "{out}");
}

#[test]
fn headings_are_quoted() {
    let f = Fixture::new();
    f.dir("d\u{1b}[1m").file("d\u{1b}[1m/f", 1, day(1));
    assert_eq!(
        one(&f, &["-R", "-Q", "d\u{1b}[1m"]),
        "\"d\\033[1m\":\n\"f\"\n"
    );
    assert_eq!(
        one(&f, &["-R", "--quoting-style=shell-escape", "d\u{1b}[1m"]),
        "'d'$'\\033''[1m':\nf\n"
    );
}

#[test]
fn escape_style_escapes_spaces_in_names_but_not_in_headings() {
    let f = Fixture::new();
    f.dir("d e").file("d e/f g", 1, day(1));
    assert_eq!(one(&f, &["-R", "-b", "d e"]), "d e:\nf\\ g\n");
    assert_eq!(one(&f, &["-1", "-b", "d e"]), "f\\ g\n");
}

#[test]
fn non_utf8_names_are_preserved_when_the_filesystem_allows_them() {
    let f = Fixture::new();
    let name = OsStr::from_bytes(b"bad\xff.txt");
    // APFS rejects such names; there is nothing to test there
    if std::fs::write(f.root().join(name), b"x").is_err() {
        return;
    }
    assert_eq!(eka(&f, &["-1"]).stdout, b"bad\xff.txt\n");
    assert_eq!(text(&eka(&f, &["-1", "-b"]).stdout), "bad\\377.txt\n");
    assert_eq!(text(&eka(&f, &["-1", "-q"]).stdout), "bad?.txt\n");
}

// --------------------------------------------------------- errors / exit

#[test]
fn missing_operand_exits_2_with_a_quoted_message() {
    let f = standard_tree();
    let out = eka(&f, &["nope"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        text(&out.stderr),
        "eka: cannot access 'nope': No such file or directory\n"
    );
    assert!(out.stdout.is_empty());
}

#[test]
fn hostile_operand_is_escaped_in_the_message() {
    let out = eka(&standard_tree(), &["no\u{1b}[2Jsuch"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        text(&out.stderr),
        "eka: cannot access 'no'$'\\033''[2Jsuch': No such file or directory\n"
    );
}

#[test]
fn good_operands_are_still_listed_when_another_is_missing() {
    let f = standard_tree();
    let out = eka(&f, &["-1", "nope", "a.txt"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(text(&out.stdout), "a.txt\n");
}

#[test]
fn unreadable_directories() {
    if running_as_root() {
        return; // root reads everything
    }
    let f = Fixture::new();
    f.dir("perm/locked").file("perm/ok", 1, day(1));
    f.mode("perm/locked", 0o000);

    // as an operand: serious problem
    let out = eka(&f, &["perm/locked"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        text(&out.stderr),
        "eka: cannot open directory 'perm/locked': Permission denied\n"
    );

    // found while listing: minor problem, the rest is still printed
    let out = eka(&f, &["-1", "-R", "perm"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stdout).starts_with("perm:\nlocked\nok\n"),
        "{}",
        text(&out.stdout)
    );
    assert!(text(&out.stderr).contains("cannot open directory 'perm/locked': Permission denied"));
}

#[test]
fn invalid_option_values_fail_with_status_2() {
    let f = standard_tree();
    for args in [
        &["--block-size=0"][..],
        &["--block-size=foo"],
        &["--block-size="],
        &["--time-style=+%Q"],
        &["--time-style=bogus"],
        &["--quoting-style=nope"],
        &["--sort=bad"],
        &["--color=sometimes"],
        &["--no-such-option"],
        &["-w", "abc"],
    ] {
        let out = eka(&f, args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(!out.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn help_and_version() {
    let f = standard_tree();
    let help = ok(&eka(&f, &["--help"]));
    assert!(help.contains("--quoting-style") && help.contains("--group-directories-first"));
    assert!(ok(&eka(&f, &["--version"])).starts_with("eka "));
    // -h is human-readable, not help
    assert!(ok(&eka(&f, &["-lh"])).starts_with("total "));
}

// ------------------------------------------------------------- recursion

fn rtree() -> Fixture {
    let f = Fixture::new();
    f.dir("a/x")
        .dir("b")
        .file("b/f", 1, day(1))
        .file("c", 1, day(1))
        .link("a", "sl")
        .dir(".hid")
        .file(".hid/z", 1, day(1));
    f
}

#[test]
fn recursive_listing_order_and_headings() {
    assert_eq!(
        ok(&eka(&rtree(), &["-1", "-R"])),
        ".:\na\nb\nc\nsl\n\n./a:\nx\n\n./a/x:\n\n./b:\nf\n"
    );
}

#[test]
fn recursive_includes_hidden_directories_with_almost_all() {
    assert_eq!(
        ok(&eka(&rtree(), &["-1", "-AR"])),
        ".:\n.hid\na\nb\nc\nsl\n\n./.hid:\nz\n\n./a:\nx\n\n./a/x:\n\n./b:\nf\n"
    );
}

#[test]
fn recursive_follows_links_only_with_dereference() {
    let out = ok(&eka(&rtree(), &["-1", "-LR"]));
    assert_eq!(
        out,
        ".:\na\nb\nc\nsl\n\n./a:\nx\n\n./a/x:\n\n./b:\nf\n\n./sl:\nx\n\n./sl/x:\n"
    );
}

#[test]
fn recursive_with_directory_flag_does_not_recurse() {
    // like ls: no heading and no recursion, just the directory itself
    assert_eq!(ok(&eka(&rtree(), &["-1", "-dR"])), ".\n");
}

#[test]
fn dereference_cycles_terminate_and_are_reported() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let f = Fixture::new();
    f.dir("cyc/sub")
        .link("..", "cyc/sub/up")
        .link(".", "cyc/self");
    let mut child = Command::new(EKA)
        .args(["-1", "-LR", "cyc"])
        .current_dir(f.root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Without cycle detection this never ends; fail in seconds instead of hanging
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(10) {
            child.kill().unwrap();
            panic!("-LR did not terminate on a directory cycle");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "cyc:\nself\nsub\n\ncyc/sub:\nup\n");
    let err = text(&out.stderr);
    assert!(
        err.contains("eka: 'cyc/self': not listing already-listed directory\n"),
        "{err}"
    );
    assert!(
        err.contains("eka: 'cyc/sub/up': not listing already-listed directory\n"),
        "{err}"
    );
}

#[test]
fn deep_trees_do_not_overflow_the_stack() {
    let f = Fixture::new();
    let mut rel = String::new();
    for _ in 0..200 {
        rel.push_str("d/");
    }
    f.dir(rel.trim_end_matches('/'));
    let out = eka(&f, &["-R", "-1"]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&out.stdout).matches(':').count(), 201);
}

// ------------------------------------------------------------ broken pipe

#[test]
fn closed_pipe_is_not_an_error() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let f = Fixture::new();
    for i in 0..4000 {
        f.file(&format!("file-with-a-fairly-long-name-{i:05}"), 1, day(1));
    }
    let mut child = Command::new(EKA)
        .args(["-1", "-R"])
        .current_dir(f.root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    drop(reader); // close the read end while eka still has output to write
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", text(&out.stderr));
    assert!(out.stderr.is_empty(), "{}", text(&out.stderr));
}

#[test]
fn device_files_show_major_and_minor_instead_of_a_size() {
    let f = standard_tree();
    let out = eka(&f, &["-l", "/dev/null"]);
    let line = text(&out.stdout);
    let fields: Vec<&str> = line.split_whitespace().collect();
    assert!(fields[4].ends_with(','), "{line}");
    assert!(fields[5].parse::<u64>().is_ok(), "{line}");
}

#[test]
fn f_lists_hidden_entries_unsorted_with_dots() {
    let f = standard_tree();
    let mut got = names(&f, &["-f"]);
    got.sort();
    let mut want = strs(&VISIBLE);
    want.extend(strs(&[".", "..", ".hdir", ".hidden"]));
    want.sort();
    assert_eq!(got, want);
}

#[test]
fn file_type_omits_the_star_of_classify() {
    let f = standard_tree();
    let out = text(&eka(&f, &["-1", "--file-type"]).stdout);
    assert!(out.lines().any(|l| l == "sub/"), "{out}");
    assert!(!out.contains('*'), "{out}");
}

#[test]
fn si_uses_powers_of_1000() {
    let f = standard_tree();
    let out = text(&eka(&f, &["-l", "--si", "big.bin"]).stdout);
    assert!(out.contains("k ") || out.contains("M "), "{out}");
}
