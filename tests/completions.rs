//! The shell completions in `completions/` are generated from the command-line
//! definition and checked in so the release archives can include them. These
//! tests fail when they are out of date: run `scripts/gen-completions.sh`.

mod common;

use common::EKA;
use std::path::Path;
use std::process::Command;

fn generated(shell: &str) -> String {
    let out = Command::new(EKA)
        .args(["--completions", shell])
        .output()
        .unwrap();
    assert!(out.status.success(), "{shell}");
    String::from_utf8(out.stdout).unwrap()
}

fn checked_in(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("completions")
        .join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn checked_in_completions_are_up_to_date() {
    for (shell, file) in [("bash", "eka.bash"), ("zsh", "_eka"), ("fish", "eka.fish")] {
        assert!(
            generated(shell) == checked_in(file),
            "completions/{file} is out of date: run scripts/gen-completions.sh"
        );
    }
}

#[test]
fn completions_offer_the_option_values() {
    let fish = generated("fish");
    for word in ["extension", "single-column", "file-type", "always", "birth"] {
        assert!(fish.contains(word), "{word}");
    }
    let zsh = generated("zsh");
    assert!(zsh.starts_with("#compdef eka"));
}

#[test]
fn unknown_shell_is_rejected() {
    let out = Command::new(EKA)
        .args(["--completions", "nushell"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}
