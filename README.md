# Eka - Modern `ls` Replacement

Eka is a drop-in replacement for the GNU/POSIX `ls` command, written in Rust. It aims to match `ls` in flags and output while improving memory use, speed and safety, and it adds color by default on terminals.

> **Status:** v0.2.0, work in progress. Listing, sorting, columns, the long format and time options are implemented; name quoting and a few GNU options are still missing. See [docs/ROADMAP.md](docs/ROADMAP.md).

## Features

- **GNU-compatible flags and output**: `-1 -C -x -m -w -a -A -d -F -p -h -i -s -k -l -n -g -o -G -L -H -P -R -r -S -t -u -c --sort --time --time-style --full-time --block-size --color`
- **Columns**: multi-column output on terminals (`-C`, `-x`, `-m`, `-w`), one entry per line when piped
- **Real owner and group names**, cached per run, with numeric fallback
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
| `-1` | One entry per line (default when output is not a terminal) |
| `-C` | List entries in columns, sorted down (default on a terminal) |
| `-x` | List entries in rows instead of columns |
| `-m` | Comma separated list, wrapped to the width |
| `-w`, `--width=COLS` | Set the output width; `0` means no limit. Default: `$COLUMNS`, then the terminal, then 80 |
| `-l` | Long listing format |
| `-n`, `--numeric-uid-gid` | Like `-l`, with numeric user and group IDs |
| `-g` | Like `-l`, without the owner |
| `-o` | Like `-l`, without the group |
| `-G`, `--no-group` | With `-l`, do not print the group |
| `-a`, `--all` | Do not ignore entries starting with `.`; also show `.` and `..` |
| `-A`, `--almost-all` | Like `-a`, without `.` and `..` |
| `-d`, `--directory` | List directories themselves, not their contents |
| `-h`, `--human-readable` | With `-l`, print sizes as `1.5K`, `234M`, `2.0G` (rounded up) |
| `-F`, `--classify` | Append an indicator (`/` directory, `*` executable, `@` symlink, `\|` fifo, `=` socket) |
| `-i`, `--inode` | Print the inode number of each entry |
| `-s`, `--size` | Print the allocated size of each entry in blocks (1 KiB by default); also prints `total` |
| `-k`, `--kibibytes` | Use 1024-byte blocks (the default) |
| `--block-size=SIZE` | Scale sizes by `SIZE`: `K M G T P E` are powers of 1024, `KB MB GB ...` powers of 1000; a bare unit means one of it. `-h` overrides it |
| `-p`, `--indicator-style=slash` | Append `/` to directories; `-F` takes precedence |
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
| `--time=WORD` | Show and sort by `atime` (`access`, `use`), `ctime` (`status`) or `mtime` (`modification`) |
| `--time-style=STYLE` | `full-iso`, `long-iso`, `iso`, `locale` or `+FORMAT` (a `strftime` format; a newline splits the recent and old formats). `$TIME_STYLE` is the default |
| `--full-time` | Like `-l --time-style=full-iso` |
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

- Not implemented yet: `-q -b -N -Q` and `ls`-style quoting of unusual names, `-v -X -U -I --hide --group-directories-first`, `--author`, `--si`, `-D`, `-Z`.
- Names are not escaped, so a name with control characters is written as is.
- `-C` pads with spaces; GNU may use tabs.
- Column widths count code points, so wide (CJK) characters are not measured correctly.
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
    ├── formatter.rs      # Short and long output, sizes, dates, totals, owner names
    ├── layout.rs         # Column layout arithmetic (-C, -x, -m)
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
