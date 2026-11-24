# Contributing to eka

Thanks for helping. Eka is a drop-in replacement for GNU `ls`, so the guiding
rule is simple: **when in doubt, do what GNU `ls` does.** Bug reports, GNU
differences you find, tests, docs and packaging are all welcome.

## Ways to help

- **Report a difference from GNU `ls`.** Open an
  [issue](https://github.com/alexlm78/eka/issues) with the exact command, your
  OS, and both outputs (eka and GNU `ls`). A file name with odd characters? Show
  it with `ls -b`.
- **Pick a roadmap item.** See [docs/ROADMAP.md](docs/ROADMAP.md). Small
  self-contained ones are a good start: `--author`, `--zero`, `--sort=width`,
  `--hyperlink`.
- **Fix a known difference.** They are listed in the
  [README](README.md#known-differences-from-gnu-ls).
- **Add tests, improve docs, or help with packaging** (`.deb`, `.rpm`, AUR).

For a large change, open an issue first so we can agree on the approach before
you write code.

## Setup

You need Rust 1.88 or later (edition 2024) on macOS or Linux. Windows is not
supported.

```bash
git clone https://github.com/alexlm78/eka.git
cd eka
cargo build
cargo run -- -l          # run eka from the checkout
```

## Project layout

| Path | Role |
|------|------|
| `src/main.rs` | Entry point, argument classification, exit status |
| `src/cli.rs` | Option parsing (clap) and sort/time/deref semantics |
| `src/directory.rs` | Directory reading, filtering, sorting, recursion |
| `src/formatter.rs` | Short and long output, sizes, dates, totals, owner names |
| `src/layout.rs` | Column layout arithmetic (`-C`, `-x`, `-m`) |
| `src/quoting.rs` | Name quoting and escaping |
| `src/collate.rs` | Locale-aware name order (macOS and glibc models) |
| `src/colors.rs` | Palette, `LS_COLORS` parsing, color decision |
| `src/glob.rs` | Shell pattern matcher for `-I` and `--hide` |
| `src/types.rs` | `FileEntry` and the file type model |
| `docs/` | Roadmap, architecture notes, man page (`eka.1`) |
| `completions/` | Generated shell completions (do not edit by hand) |
| `scripts/` | Version check, completions, benchmarks, packaging |

## Tests

There are three layers; a change normally touches one or more of them.

1. **Unit tests** next to the code.
2. **Behavior tests** (`tests/behavior.rs`): run the binary and compare with
   hand-written expected output.
3. **Compatibility tests** (`tests/compat_ls.rs`): compare eka with a reference
   `ls`.

```bash
cargo test
cargo clippy --all-targets
cargo fmt --all
```

The compatibility tests run against `/bin/ls`. The BSD `ls` on macOS lacks many
GNU options, so those comparisons only run when the reference is GNU coreutils.
Point `EKA_TEST_LS` at one:

```bash
brew install coreutils
EKA_TEST_LS=/opt/homebrew/bin/gls cargo test --test compat_ls
```

On Linux, `/bin/ls` is already GNU. If you change sorting, quoting or layout,
please run the compatibility tests on GNU `ls`.

## Rules the project follows

CI enforces most of these.

### Versions and the changelog

- **Every commit that changes eka takes the next odd patch number**
  (0.5.9, 0.5.11, 0.5.13, ...). Even numbers are not used.
- The same commit adds a `## [X.Y.Z] - YYYY-MM-DD` section to
  [CHANGELOG.md](CHANGELOG.md) (Added / Changed / Fixed) and updates `Cargo.toml`
  and `Cargo.lock`. Do not use `[Unreleased]`.
- Check it locally with `scripts/check-version.sh`.
- A release is a tag `vX.Y.Z` on the commit that has that version. Maintainers
  do the tagging.

### Code

- **No `unsafe`.** The crate has `#![forbid(unsafe_code)]`. If a feature seems to
  need it (macOS ACLs, for example), open an issue first.
- Code must be `cargo fmt` clean and pass `cargo clippy --all-targets -- -D warnings`.
- **Names are bytes.** Do not assume file names are valid UTF-8, and never print
  a name to a terminal without going through `quoting`.
- **No panics on odd input.** Bad timestamps, huge values and invalid formats
  must degrade (print `?`, report an error), not abort the listing.
- The last option wins when options compete. Keep that behavior when you add
  options.
- Do not add dependencies without a good reason; each one costs start-up time
  and audit surface.
- Keep the minimum Rust version (`rust-version` in `Cargo.toml`) building.

### When you change the command line

- Run `scripts/gen-completions.sh` to refresh `completions/`. `cargo test` fails
  when they are out of date.
- Update the option table in the [README](README.md), the man page
  (`docs/eka.1`) and the web docs (`web/docs.html`).

### When you change behavior

- Add or update a test that fails without your change.
- Compare with GNU `ls` and say in the PR which version you checked
  (`ls --version`).
- If eka differs on purpose, document it under *Known differences* in the README.

### Performance

Claims about speed need numbers. Use `scripts/bench.sh` (needs
[hyperfine](https://github.com/sharkdp/hyperfine)) and include before/after
results in the PR. Do not add parallelism or caching that does not measure
faster.

## Pull requests

1. Fork the repository and create a branch from `main`.
2. Make your change with tests, the version bump and the changelog entry.
3. Run `cargo fmt --all`, `cargo clippy --all-targets` and `cargo test`.
4. Open a pull request that says **what** changed and **why**, and how you
   checked it against GNU `ls`.

CI runs on Linux (GNU `ls`) and macOS and also checks formatting, clippy, a
minimum-Rust-version build, a dependency audit (`cargo audit`), the version and
changelog, and the man page (`mandoc -Tlint`). All must pass.

Commit messages use a short conventional prefix (`feat:`, `fix:`, `docs:`,
`test:`, `refactor:`, `perf:`), for example `fix: sort $ like glibc on Linux`.

## The website

The project site lives in `web/` as plain HTML and CSS, with no build step.
Open `web/index.html` in a browser to preview it. Keep it in sync with the
README when options or install steps change.

## License

By contributing you agree that your work is released under the
[MIT License](LICENSE).
