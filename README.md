# Eka - Modern `ls` Replacement

Eka is a drop-in replacement for the GNU/POSIX `ls` command, written in Rust. It aims to match `ls` in flags and output while improving memory use, speed and safety, and it adds color by default on terminals.

> **Status:** v0.3.0, the first public release. It is not 1.0 yet: names sort by an approximation of GNU `ls` collation (see [Known differences](#known-differences-from-gnu-ls)), and a few GNU options are missing. See [Known differences](#known-differences-from-gnu-ls) and the [CHANGELOG](CHANGELOG.md).

## Features

- **GNU-compatible flags and output**: `-1 -C -x -m -w -a -A -B -d -F -p -h -i -s -k -l -n -g -o -G -L -H -P -R -r -S -t -u -c -v -X -U -I --hide --group-directories-first --sort --time --time-style --full-time --block-size --color -N -b -Q -q --show-control-chars --quoting-style`
- **Columns**: multi-column output on terminals (`-C`, `-x`, `-m`, `-w`), one entry per line when piped
- **Real owner and group names**, cached per run, with numeric fallback
- **Correct long format**: no header row, columns sized to fit, `total` in 1 KiB blocks, `Mon DD HH:MM` for recent files and `Mon DD  YYYY` for older ones
- **Faithful file model**: symlinks are not followed unless asked (`-L`, `-H`), broken links are listed, and fifo, socket and device files show the right type and setuid/setgid/sticky bits
- **Safe output**: on a terminal, names are quoted so that control characters, escape sequences and bidirectional overrides cannot alter what you see; names that are not valid UTF-8 are preserved (see [Safe output](#safe-output))
- **Color**: automatic on terminals, honors `NO_COLOR` and `LS_COLORS`
- **Buffered output**: one locked, buffered write path; a closed pipe (`eka | head`) exits quietly
- **Unix only**: macOS and Linux

## Installation

### From a release

Download the archive for your system from the [releases page](https://github.com/alexlm78/eka/releases) (macOS and Linux, x86-64 and ARM64), check it against `SHA256SUMS`, and unpack it. Each archive holds the `eka` binary, the man page, the README, the license and the changelog:

```bash
tar -xzf eka-0.3.0-aarch64-apple-darwin.tar.gz
install -m 755 eka-0.3.0-aarch64-apple-darwin/eka /usr/local/bin/eka
install -m 644 eka-0.3.0-aarch64-apple-darwin/eka.1 /usr/local/share/man/man1/eka.1
```

Release archives carry a build-provenance attestation; verify one with `gh attestation verify FILE --repo alexlm78/eka`.

### From Source

```bash
git clone https://github.com/alexlm78/eka.git
cd eka

# Build
cargo build --release

# Install the binary (optional)
cargo install --path .

# Install the man page (optional)
./install-manpage.sh
```

## Usage

```bash
eka                  # list the current directory
eka -l               # long format
eka -lh              # long format, human-readable sizes
eka -la              # include hidden files, plus . and ..
eka -lA              # include hidden files, without . and ..
eka -lSr             # smallest files first
eka -lt              # newest first
eka -R src/          # recurse into subdirectories
eka -d */            # list the directories themselves
eka -F               # append / * @ | = indicators
eka --color=always | less -R
```

When several paths are given, files are listed first, then each directory under its own `path:` heading, as `ls` does.

### Options

| Flag | Description |
|------|-------------|
| `-1` | One entry per line (default when output is not a terminal) |
| `-C` | List entries in columns, sorted down (default on a terminal) |
| `-x` | List entries in rows instead of columns |
| `-m` | Comma separated list, wrapped to the width |
| `-T`, `--tabsize=COLS` | Pad columns with tabs at every `COLS` (default 8); `0` pads with spaces only. No tabs are used with `-w 0` |
| `-w`, `--width=COLS` | Set the output width; `0` means no limit. Default: `$COLUMNS`, then the terminal, then 80 |
| `-l` | Long listing format |
| `-n`, `--numeric-uid-gid` | Like `-l`, with numeric user and group IDs |
| `-g` | Like `-l`, without the owner |
| `-o` | Like `-l`, without the group |
| `-G`, `--no-group` | With `-l`, do not print the group |
| `-f` | Do not sort and list all entries (same as `-aU`) |
| `--format=WORD` | `across` or `horizontal` (`-x`), `commas` (`-m`), `long` or `verbose` (`-l`), `single-column` (`-1`), `vertical` (`-C`). Unlike GNU ls, it does not follow "last option wins" against `-l -1 -C -x -m` |
| `-a`, `--all` | Do not ignore entries starting with `.`; also show `.` and `..` |
| `-A`, `--almost-all` | Like `-a`, without `.` and `..` |
| `-d`, `--directory` | List directories themselves, not their contents |
| `-h`, `--human-readable` | With `-l`, print sizes as `1.5K`, `234M`, `2.0G` (rounded up) |
| `--si` | Like `-h`, but with powers of 1000 (`1.5k`, `234M`) |
| `-F`, `--classify` | Append an indicator (`/` directory, `*` executable, `@` symlink, `\|` fifo, `=` socket) |
| `--file-type` | Like `-F`, without `*` |
| `-i`, `--inode` | Print the inode number of each entry |
| `-s`, `--size` | Print the allocated size of each entry in blocks (1 KiB by default); also prints `total` |
| `-k`, `--kibibytes` | Use 1024-byte blocks (the default) |
| `--block-size=SIZE` | Scale sizes by `SIZE`: `K M G T P E` and `KiB MiB ...` are powers of 1024, `KB MB GB ...` powers of 1000. A bare unit (`K`) means one of it and sizes then print with its suffix (`16K`); with a number in front (`1K`, `2K`, `1024`) they do not. `-h` overrides it |
| `-p`, `--indicator-style=STYLE` | `none`, `slash` (`-p`: append `/` to directories), `file-type` or `classify` (`-F`); `-F` takes precedence over `-p` |
| `-R`, `--recursive` | List subdirectories recursively (symlinked directories are not followed) |
| `-S` | Sort by size, largest first |
| `-t` | Sort by time, newest first |
| `-u` | Use access time. With `-lt` sort by it and show it; with `-l` show it and sort by name; otherwise sort by it |
| `-c` | Use status-change (ctime) time. Same rules as `-u` |
| `-v` | Natural sort of version numbers (`file2` before `file10`) |
| `-X` | Sort by extension, then name |
| `-U` | Do not sort; list in directory order |
| `-r`, `--reverse` | Reverse the sort order |
| `--sort=WORD` | `name` (default), `none`, `size`, `time`, `version`, `extension` |
| `--group-directories-first` | List directories (and symlinks to directories) before other entries, whatever the sort or `-r` |
| `-I`, `--ignore=PATTERN` | Do not list entries matching the shell `PATTERN`; repeatable, applies even with `-a` |
| `--hide=PATTERN` | Like `-I`, but `-a` and `-A` override it |
| `-B`, `--ignore-backups` | Do not list entries ending with `~` |
| `-L`, `--dereference` | Show information for the file a symlink points to |
| `-H`, `--dereference-command-line` | Follow symlinks named on the command line |
| `-P` | Never follow symlinks (default); overrides `-H` and `-L` |
| `--time=WORD` | Show and sort by `atime` (`access`, `use`), `ctime` (`status`) `mtime` (`modification`) or `birth` (`creation`; prints `?` when the filesystem does not record it) |
| `--time-style=STYLE` | `full-iso`, `long-iso`, `iso`, `locale` or `+FORMAT` (a `strftime` format; if it contains a newline, the part before it is used for older files and the part after it for recent ones). `$TIME_STYLE` is the default |
| `--full-time` | Like `-l --time-style=full-iso` |
| `-N`, `--literal` | Print names as they are |
| `-b`, `--escape` | Print C-style backslash escapes for unusual characters |
| `-Q`, `--quote-name` | Enclose names in double quotes, with C escapes |
| `-q`, `--hide-control-chars` | Print `?` instead of unprintable characters |
| `--show-control-chars` | Print unprintable characters as they are (the default when output is not a terminal) |
| `--quoting-style=WORD` | `literal`, `shell`, `shell-always`, `shell-escape`, `shell-escape-always`, `c` or `escape` |
| `--color[=WHEN]` | `always`, `auto` (default) or `never`; plain `--color` means `always` |
| `--help` | Print help |
| `-V`, `--version` | Print the version |

Names are sorted bytewise in the `C` locale (`LC_ALL=C`). In any other locale (`LC_ALL`, `LC_COLLATE` or `LANG`, in that order) they sort ignoring case and accents, with punctuation before digits before letters, as GNU `ls` does in a UTF-8 locale. Ties in size and time fall back to the name, and directories are not grouped first unless `--group-directories-first` is given. When several sort options are combined the order of precedence is `--sort`, `-U`, `-S`, `-t`, `-v`, `-X`.

Patterns for `-I` and `--hide` support `*`, `?`, `[abc]`, `[a-z]`, `[!abc]` and `\` escapes, are matched against the whole name (a leading dot is not special) and never apply to paths named on the command line.

### Exit status

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Minor problem, such as an unreadable entry inside a listed directory |
| 2 | Serious problem, such as a path on the command line that cannot be accessed |

## Safe output

File names are arbitrary bytes, and a name such as `evil\x1b[2J` or `report\u202etxt.exe` can clear the screen, change the window title or disguise a file type when it is printed raw. Eka follows GNU `ls` here:

- **On a terminal** the default quoting style is `shell-escape`: a name that the shell would need quoted is wrapped in single quotes, and unprintable characters appear as `$'\033'`-style escapes, so every name can be copied back into a shell. Unquoted names get one leading space when others are quoted, so the names line up.
- **When output is not a terminal** (a pipe or a file) names are written as they are, because the output is data for another program. Use `-b`, `-Q`, `-q` or `--quoting-style` to escape them anyway, or set `QUOTING_STYLE` in the environment.
- A newline in a name is always shown as `?` when names are printed as they are (`-N`, `literal`, `shell`), so one entry cannot span two lines.
- **Unprintable** means control characters (C0, DEL, C1), the bidirectional overrides and isolates (U+202A-202E, U+2066-2069, U+200E, U+200F, U+061C), the line and paragraph separators, and bytes that are not valid UTF-8.
- Link targets and the `path:` headings of `-R` and multi-path listings are quoted the same way.
- **Error messages** always quote the paths they mention, even when standard error is not a terminal.
- `-R` cannot loop: a directory that is its own ancestor (possible with `-L`) is reported as `not listing already-listed directory` and skipped.
- The program contains no `unsafe` code (`#![forbid(unsafe_code)]`), and unrepresentable timestamps print as `?` instead of aborting the listing.

## Color

Colors are used only when standard output is a terminal, unless `--color` says otherwise. With `--color=auto`, `NO_COLOR` (set and not empty) and `TERM=dumb` turn color off; `--color=always` ignores them.

If `LS_COLORS` is set, Eka uses it (keys `di ln ex or pi so bd cd fi` and `*.ext` patterns) on top of the GNU defaults. Otherwise it uses its own scheme:

| File Type | Color |
|-----------|-------|
| Directory | Cyan |
| Executable | Green |
| Symlink | Yellow |
| Broken symlink | Bold red |
| Fifo | Yellow |
| Socket | Magenta |
| Block / character device | Bold yellow |
| Image | Magenta |
| Video | Red |
| Audio | Orange |
| Archive | Blue |
| Regular file | Default |

## Performance

Eka reads a directory with a single pass and calls `stat` only for the information the output needs: names alone (`eka`, `eka -1R`, `-i`, `-U`) never touch the files, as with `ls -f`, while `-l`, `-s`, `-F`, color, and sorting by size or time do.

Measured with `scripts/bench.sh` (hyperfine, 15 runs) on an Apple M-series Mac with 10 cores, APFS, macOS and Homebrew GNU coreutils; one fixture with 20,000 files in one directory and one with 20,000 files in 2,200 directories:

| Case | eka | GNU ls | BSD ls | eza |
|------|-----|--------|--------|-----|
| `-1`, 20,000 files | **22 ms**, 15 MB | 67 ms, 8 MB | 88 ms, 4 MB | 137 ms, 48 MB |
| `-l`, 20,000 files | **85 ms**, 22 MB | 133 ms, 9 MB | 214 ms, 10 MB | 497 ms, 101 MB |
| `-R`, 2,200 directories | 74 ms, 14 MB | 74 ms, 2 MB | 228 ms, 1 MB | 202 ms, 8 MB |
| start-up, 30 files | 4.5 ms, 6 MB | 3.2 ms, 2 MB | 2.3 ms, 1 MB | 8.7 ms, 7 MB |

Eka is faster than the others on large flat listings and ties GNU ls on recursion, but it **uses more memory** (it keeps every entry of a directory before sorting, where `ls` streams the unsorted case) and **starts slower** than the C implementations (a Rust program that does nothing starts in about 2.6 ms on this machine; the remaining ~2 ms is eka's option parsing and set-up). Results depend on the file system and on the cache; run `scripts/bench.sh` to measure your own.

## Known differences from GNU `ls`

- **Newlines in names:** with `--quoting-style=literal` or `shell` (and `-N`), eka prints a newline inside a name as `?`. GNU coreutils 9.4 prints it raw; newer versions print `?`. The escaping styles (`-b`, `-Q`, `shell-escape`, `c`, `escape`) are identical.
- **Column fit:** eka lets the columns fill the width exactly (`total <= width`), as recent coreutils do. Older ones, such as Ubuntu's 9.4, need `total < width`, so at exact-fit widths they use one column fewer.
- **Sort order:** in the `C` locale names compare bytewise, like GNU `ls`. In other locales eka approximates the GNU order: case and accents are ignored first, then accents and case (lowercase first) break ties. It matches GNU `ls` on Latin-script names, ASCII punctuation, digits and accented Latin-1 letters (checked against 300 random names). Other scripts and language-specific rules (such as Spanish `ch` or German `ß`) sort by code point.

- Not implemented yet: `--author`, `-D`, `-Z`.
- Shell quoting always uses single quotes (`'it'\\''s'`), where GNU uses double quotes for a name with an apostrophe. The `locale` and `clocale` quoting styles are not implemented.
- With `--quoting-style=shell`, a name whose only special character is a control character is quoted by eka and left bare by GNU.
- Bidirectional overrides (such as U+202E) are always treated as unprintable. GNU decides by locale, and in a UTF-8 locale it prints them as they are.
- Column widths count code points, so wide (CJK) characters are not measured correctly.
- `total` counts 1 KiB blocks, as GNU does; the BSD/macOS `ls` counts 512-byte blocks.
- **Attribute markers:** on macOS `-l` prints `@` after the mode of a file with extended attributes, like the system `ls`; the `+` for an ACL is not printed, because reading ACLs needs unsafe code. On Linux it prints `+` (POSIX ACL) and `.` (SELinux context), like GNU `ls`. The marker column is added only when some file in the listing has a marker, and the other files are padded.

## Project Structure

```
eka/
├── Cargo.toml
├── README.md
├── LICENSE
├── install-manpage.sh
├── docs/
│   ├── ROADMAP.md
│   ├── eka-architecture.md
│   └── eka.1             # man page
└── src/
    ├── main.rs           # Entry point, argument classification, exit status
    ├── cli.rs            # Option parsing (clap) and sort/time/deref semantics
    ├── directory.rs      # Directory reading, filtering, sorting, recursion
    ├── formatter.rs      # Short and long output, sizes, dates, totals, owner names
    ├── layout.rs         # Column layout arithmetic (-C, -x, -m)
    ├── glob.rs           # Shell pattern matcher for -I and --hide
    ├── quoting.rs        # Name quoting and escaping (-b -Q -q --quoting-style)
    ├── colors.rs         # Palette, LS_COLORS parsing, color decision
    └── types.rs          # FileEntry (lstat based) and file type model
```

## Development

The test suite has three layers: unit tests next to the code, end-to-end tests of the binary with hand-written expected output (`tests/behavior.rs`), and comparisons with the system `ls` (`tests/compat_ls.rs`).

```bash
cargo build
cargo test
cargo clippy --all-targets
```

The comparison tests run against `/bin/ls`. The BSD `ls` of macOS lacks many GNU options, so those comparisons run only when the reference is GNU coreutils. Point `EKA_TEST_LS` at another `ls` to check against it, for example GNU `ls` installed by Homebrew:

```bash
EKA_TEST_LS=/opt/homebrew/bin/gls cargo test --test compat_ls
```

CI runs the suite on Linux (GNU `ls`) and macOS, plus a dependency audit and a minimum-Rust-version build.

Requires Rust 1.88 or later (edition 2024 with let-chains) on macOS or Linux. The minimum is checked in CI (`rust-version` in `Cargo.toml`).

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
