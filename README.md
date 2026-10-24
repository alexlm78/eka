# Eka - Modern `ls` Replacement

Eka is a drop-in replacement for the GNU/POSIX `ls` command, written in Rust. It aims to match `ls` in flags and output while improving memory use, speed and safety, and it adds color by default on terminals.

> **Status:** v0.2.0, work in progress. The core flags below are implemented; column output, owner/group names and several GNU options are still missing. See [docs/ROADMAP.md](docs/ROADMAP.md).

## Features

- **GNU-compatible flags and output**: `-1 -a -A -d -F -h -i -l -L -H -P -R -r -S -t -u -c --sort --color`
- **Correct long format**: no header row, columns sized to fit, `total` in 1 KiB blocks, `Mon DD HH:MM` for recent files and `Mon DD  YYYY` for older ones
- **Faithful file model**: symlinks are not followed unless asked (`-L`, `-H`), broken links are listed, and fifo, socket and device files show the right type and setuid/setgid/sticky bits
- **Raw file names**: names that are not valid UTF-8 are written byte for byte
- **Color**: automatic on terminals, honors `NO_COLOR` and `LS_COLORS`
- **Buffered output**: one locked, buffered write path; a closed pipe (`eka | head`) exits quietly
- **Unix only**: macOS and Linux

## Installation

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
| `-1` | One entry per line (currently the only layout) |
| `-l` | Long listing format |
| `-a`, `--all` | Do not ignore entries starting with `.`; also show `.` and `..` |
| `-A`, `--almost-all` | Like `-a`, without `.` and `..` |
| `-d`, `--directory` | List directories themselves, not their contents |
| `-h`, `--human-readable` | With `-l`, print sizes as `1.5K`, `234M`, `2.0G` (rounded up) |
| `-F`, `--classify` | Append an indicator (`/` directory, `*` executable, `@` symlink, `\|` fifo, `=` socket) |
| `-i`, `--inode` | Print the inode number of each entry |
| `-R`, `--recursive` | List subdirectories recursively (symlinked directories are not followed) |
| `-S` | Sort by size, largest first |
| `-t` | Sort by time, newest first |
| `-u` | Use access time. With `-lt` sort by it and show it; with `-l` show it and sort by name; otherwise sort by it |
| `-c` | Use status-change (ctime) time. Same rules as `-u` |
| `-r`, `--reverse` | Reverse the sort order |
| `--sort=WORD` | `name` (default), `none`, `size`, `time`, `extension` |
| `-L`, `--dereference` | Show information for the file a symlink points to |
| `-H`, `--dereference-command-line` | Follow symlinks named on the command line |
| `-P` | Never follow symlinks (default); overrides `-H` and `-L` |
| `--color[=WHEN]` | `always`, `auto` (default) or `never`; plain `--color` means `always` |
| `--help` | Print help |
| `-V`, `--version` | Print the version |

Names are sorted bytewise (like `LC_ALL=C ls`), ties in size and time fall back to the name, and directories are not grouped first.

### Exit status

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Minor problem, such as an unreadable entry inside a listed directory |
| 2 | Serious problem, such as a path on the command line that cannot be accessed |

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

## Known differences from GNU `ls`

- Owner and group are shown as numeric IDs until name lookup lands.
- No multi-column output yet, so the default layout is one entry per line even on a terminal.
- Not implemented yet: `-n -g -o -s -p -q -b -N -Q -m -x -C -v -X -U -I --hide --time --time-style --full-time --group-directories-first --block-size` and `ls`-style quoting of unusual names.
- `total` counts 1 KiB blocks, as GNU does; the BSD/macOS `ls` counts 512-byte blocks.
- Extended attribute and ACL markers (`@`, `+`) are not printed.

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
    ├── formatter.rs      # Short and long output, sizes, dates, totals
    ├── colors.rs         # Palette, LS_COLORS parsing, color decision
    └── types.rs          # FileEntry (lstat based) and file type model
```

## Development

```bash
cargo build
cargo test
cargo clippy --all-targets
```

Requires Rust 1.88 or later (edition 2024, let-chains) on macOS or Linux.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
