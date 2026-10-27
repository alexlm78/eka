# Eka roadmap: a complete `ls` replacement

Eka aims to be a drop-in replacement for GNU `ls` on macOS and Linux: the same
options, the same output, the same exit status. This file tracks what is done,
what is left, and the rules the project follows. The day-to-day record of
changes is [CHANGELOG.md](../CHANGELOG.md).

## Versioning

- Every commit that changes eka takes the next **odd** patch number
  (0.5.5, 0.5.7, 0.5.9, ...). Even numbers are not used.
- The commit also adds a `## [X.Y.Z] - YYYY-MM-DD` section to `CHANGELOG.md`
  and updates `Cargo.toml` and `Cargo.lock`. The release workflow stops when
  the tag does not match `Cargo.toml` or the version has no notes, so every
  commit stays releasable.
- `scripts/check-version.sh` enforces this in CI. A release is a tag `vX.Y.Z`
  on the commit that has that version.
- Until 1.0.0, minor versions may change behavior.

## Where eka stands

Done and checked against GNU coreutils 9.x and the BSD `ls`:

- Every common option: `-1 -a -A -B -b -C -c -d -F -f -G -g -H -h -I -i -k -L
  -l -m -N -n -o -P -p -Q -q -R -r -S -s -T -t -U -u -v -w -x -X`, and the long
  options `--block-size --color --completions --file-type --format
  --full-time --group-directories-first --hide --indicator-style
  --quoting-style --show-control-chars --si --sort --time --time-style`.
- The last option wins when options compete, as in GNU `ls`.
- Locale-aware name order (macOS model), device numbers, extended-attribute
  and ACL markers, `LS_COLORS`, `QUOTING_STYLE`, `TIME_STYLE`, safe output on a
  terminal, no `unsafe` code.
- Man page, shell completions (bash, zsh, fish), release archives with
  provenance attestations, a Homebrew formula template.

## Phase 0: stabilize (blocks the rest)

CI must be green on Linux and macOS before anything new lands.

- [x] Format, lock file and version fixes.
- [ ] Name order on Linux. glibc ignores punctuation at the first level, so
      `.h` sorts as `h`; the current model follows macOS. Take the reference
      from Ubuntu's `ls`, fit a glibc model, and fall back to ICU4X
      (`icu_collator`) if it cannot reach 99% agreement on the random corpus.
- [ ] `-U`/`-f` list `.` and `..` where `readdir` returns them; eka puts them
      first. Not fixable with Rust's `read_dir`: drop them from the comparison
      tests and document it.
- [ ] Linux-only paths that CI has never run: POSIX ACL `+`, device numbers,
      `--time=birth`.
- [x] Version check in CI (`scripts/check-version.sh`).

## Phase 1: release 0.5.5

- [x] Release 0.5.5 on GitHub.
- [ ] First release that publishes `eka-ls` to crates.io and the Homebrew tap
      (`alexlm78/homebrew-tap`); check `brew install eka-ls` on macOS and
      Linux, and `cargo install eka-ls`.

## Phase 2: fidelity with GNU `ls`

In order of value:

1. `LSCOLORS`, `CLICOLOR` and `CLICOLOR_FORCE` (macOS), and the color of a
   symbolic link's target in `-l`. Decide whether the palette without
   `LS_COLORS` follows the GNU defaults.
2. Quoting: double quotes for names with an apostrophe in the shell styles,
   and the `locale` and `clocale` styles.
3. Small options: `--author`, `--sort=width`, `--zero`, `--hyperlink`.
4. Display width of wide (CJK) and combining characters in the column layouts.
5. Timestamps: the six-month window, future dates, and the `locale` style in
   locales other than `C`.

## Phase 3: platform parity

- [ ] `-Z`, `--context` on Linux (SELinux context, read with `xattr`).
- Out of scope unless requested: macOS ACL `+` (needs `unsafe` or a heavy
  dependency for one character), `--dired` and `-D` (Emacs only).

## Phase 4: robustness

- [ ] A nightly differential job: random trees and random option sets against
      `gls` on macOS and coreutils on Ubuntu.
- [ ] Parity of error messages and exit status (permissions, `--`, loops).
- [ ] Benchmarks against `ls` and `eza` on 100,000 files; parallelize only
      what measures slower.
- [ ] A locale that is set but not installed falls back to `C`, as in GNU.
- [ ] Property tests on `quoting`: no raw control character reaches a terminal.

## Phase 5: distribution and documentation

- [ ] Publish on crates.io (check that the name is free).
- [ ] Static musl build, `.deb` and `.rpm` packages, an AUR `PKGBUILD`.
- [ ] Refresh `docs/eka-architecture.md` (modules `collate`, `attrs`, the
      option resolution in `cli`) and add a migration guide (`alias ls=eka`).

## Criteria for 1.0.0

- Every known difference from GNU `ls` is fixed or listed as intentional.
- The nightly differential job has been clean for several weeks.
- CI is green on macOS and Linux, and the command line is stable.

## Known differences

Listed in the [README](../README.md#known-differences-from-gnu-ls); this file
only schedules the work.
