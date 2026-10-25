# Changelog

All notable changes to eka are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/). Before 1.0.0, minor versions may
change behavior.

## [Unreleased]

### Changed

- In a collating locale (`LC_ALL`, `LC_COLLATE` or `LANG` other than `C`,
  `POSIX` or `C.*`) names sort ignoring case and accents, with punctuation
  before digits before letters, as GNU ls does in a UTF-8 locale. The `C`
  locale keeps the bytewise order.
- `-X` compares from the last dot of the name, dot included, as GNU ls does,
  so ".bashrc" has the extension ".bashrc".

### Added

- `-l` prints `@` after the mode of a file with extended attributes on macOS,
  and `+` (POSIX ACL) or `.` (SELinux context) on Linux, as the system `ls`
  does. The marker column appears only when some file in the listing has a
  marker. Reading attributes costs one more system call per file in the long
  format (about 3 ms per 1,000 files). macOS ACLs (`+`) are not reported.
- `-f`, `--format`, `--file-type`, `--si`, `--indicator-style=none|file-type|classify`
  and `--time=birth` (`creation`).

### Fixed

- `-l` shows `major, minor` for character and block devices instead of `0`.
- Long options can be abbreviated when unambiguous (`--recurs`), and so can
  the values of `--sort`, `--time`, `--quoting-style` and `--color`
  (`--sort=t`), as in GNU ls.

## [0.3.0] - 2026-10-24

First public release. Eka is now a close GNU `ls` replacement for macOS and
Linux, verified against both the BSD `ls` and GNU coreutils.

### Added

- GNU-compatible options: `-1 -a -A -B -b -C -c -d -F -G -g -H -h -I -i -k -L
  -l -m -N -n -o -P -p -Q -q -R -r -S -s -T -t -U -u -v -w -X -x`, plus
  `--block-size`, `--color`, `--full-time`, `--group-directories-first`,
  `--hide`, `--quoting-style`, `--show-control-chars`, `--sort`, `--time` and
  `--time-style`.
- Multi-column output (`-C`, `-x`, `-m`, `-w`, `-T`) that uses the terminal
  width, and one name per line when output is not a terminal.
- Real owner and group names, resolved once per run.
- `LS_COLORS`, `NO_COLOR`, `QUOTING_STYLE`, `TIME_STYLE` and `COLUMNS` support.
- A man page (`docs/eka.1`).
- End-to-end tests of the binary, and comparison tests against the system `ls`
  (`EKA_TEST_LS` selects the reference).
- CI on Linux and macOS, a minimum-Rust-version build, a dependency audit and
  a release workflow.

### Changed

- **Breaking:** `-h` prints human-readable sizes and `-H` follows symbolic
  links on the command line, as in GNU ls. The `--sort_size`, `--sort_time`,
  `--sort_access` and `--directories` options are gone in favor of the GNU
  names.
- **Breaking:** the long format has no header row, sizes the columns to their
  content, and `total` counts allocated 1 KiB blocks. Directories are no longer
  listed first unless `--group-directories-first` is given.
- **Breaking:** on a terminal, names that need quoting are shown in single
  quotes or with `$'...'` escapes. Use `-N` for the old output.
- **Breaking:** color is used only when standard output is a terminal, unless
  `--color` says otherwise.
- `-d` lists directories themselves, `-a` shows `.` and `..`, and a file named
  on the command line is listed instead of rejected.
- Exit status is 2 for an argument that cannot be accessed and 1 for a problem
  found while listing a directory.
- Listing no longer calls `stat` for every entry when the output only needs
  names, so `eka -1` on a directory with 20,000 files takes 22 ms instead of
  70 ms, and `eka -R` on 2,200 directories takes 74 ms instead of 123 ms.
- The release binary is built with link-time optimization and stripped.

### Fixed

- Symbolic links are no longer followed by default, and broken links are
  listed instead of dropped.
- File types, permission bits (setuid, setgid, sticky), status-change time and
  link counts are read correctly; file kinds are detected from the extension,
  not the whole name.
- Names that are not valid UTF-8 are written byte for byte.
- A closed pipe (`eka | head`) exits quietly.
- `-R` cannot loop on a directory that is its own ancestor.
- Invalid `--time-style` formats and out-of-range timestamps no longer abort
  the program.

### Security

- Names are escaped on a terminal, so control characters, escape sequences and
  bidirectional overrides cannot rewrite what the user sees. Paths in error
  messages are always escaped.
- The program contains no `unsafe` code.

### Known differences from GNU ls

- Names are sorted bytewise, as in the `C` locale. GNU ls in a UTF-8 locale
  sorts case-insensitively and by collation rules, so listings can differ.
- Not implemented: `--author`, `--si`, `-D`, `-Z`, the `locale` and `clocale`
  quoting styles, and the extended-attribute and ACL markers in `-l`.
- Shell quoting always uses single quotes; GNU ls uses double quotes for a name
  with an apostrophe.
- Windows is not supported.

[0.3.0]: https://github.com/alexlm78/eka/releases/tag/v0.3.0
