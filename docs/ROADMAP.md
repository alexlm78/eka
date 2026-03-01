# Eka Roadmap - Complete `ls` Replacement

This document outlines the development roadmap to transform Eka into a fully functional replacement for the traditional `ls` command.

## Current Status

### Implemented Features ✅

| Feature | Status | Priority |
|---------|--------|----------|
| Short format listing | ✅ Complete | - |
| Long format listing (`-l`) | ✅ Complete | - |
| Color-coded output by file type | ✅ Complete | - |
| Hidden files support (`-a`) | ✅ Complete | - |
| Directory-only listing (`-d`) | ✅ Complete | - |
| Human-readable sizes (`-H`) | ✅ Complete | - |
| Alphabetical sorting | ✅ Complete | - |
| Directories-first sorting | ✅ Complete | - |
| Permission display | ✅ Complete | - |
| File size display | ✅ Complete | - |
| Modification date display | ✅ Complete | - |

### Missing Features (Priority Order)

## Phase 1: Core Functionality Enhancements

### 1.1 Additional Flags (High Priority)

| Flag | Description | Status |
|------|-------------|--------|
| `-1` | One file per line | 🔲 TODO |
| `-C` | Multi-column output (default) | 🔲 TODO |
| `-F` | Classify indicators (*/=>@\|) | 🔲 TODO |
| `-R` | Recursive subdirectory listing | 🔲 TODO |
| `-S` | Sort by file size | 🔲 TODO |
| `-t` | Sort by modification time | 🔲 TODO |
| `-u` | Sort by access time | 🔲 TODO |
| `-c` | Sort by change time | 🔲 TODO |
| `--sort=WORD` | Sort by extension/version/size/status | 🔲 TODO |
| `--reverse` | Reverse sort order | 🔲 TODO |
| `-i` | Print inode number | 🔲 TODO |
| `-n` | Numeric UIDs/GIDs | 🔲 TODO |
| `-g` | Long format without owner | 🔲 TODO |
| `-o` | Long format without group | 🔲 TODO |
| `-p` | Append file type indicator | 🔲 TODO |
| `-q` | Hide control characters | 🔲 TODO |
| `-b` | Escape non-graphic characters | 🔲 TODO |
| `-N` | Print raw entry names | 🔲 TODO |
| `-k` | Use 1024-byte blocks (default) | 🔲 TODO |
| `--block-size=SIZE` | Scale sizes by SIZE | 🔲 TODO |
| `--time=WORD` | Show time as WORD | 🔲 TODO |
| `--time-style=STYLE` | Time/date format | 🔲 TODO |

### 1.2 Output Enhancements

| Feature | Description | Status |
|---------|-------------|--------|
| File type indicators | Append `*` for executables, `/` for dirs, etc. | 🔲 TODO |
| Inode numbers | Display inode in long format | 🔲 TODO |
| Block counts | Display total block count | 🔲 TODO |
| Symbolic link handling | Show -> target for symlinks | 🔲 TODO |
| Broken symlink detection | Handle broken symlinks gracefully | 🔲 TODO |

## Phase 2: Advanced Features

### 2.1 Extended Attributes

| Feature | Description | Status |
|---------|-------------|--------|
| Extended flags | `--author`, `--width`, etc. | 🔲 TODO |
| File ownership | Owner/group name resolution | 🔲 TODO |
| ACL indicators | Show + for ACL | 🔲 TODO |
| SELinux context | Display SELinux context | 🔲 TODO |

### 2.2 Performance

| Feature | Description | Status |
|---------|-------------|--------|
| Parallel directory reading | Use rayon for concurrent I/O | 🔲 TODO |
| Caching | Cache directory contents | 🔲 TODO |
| Lazy loading | Defer expensive operations | 🔲 TODO |

### 2.3 Sorting Enhancements

| Feature | Description | Status |
|---------|-------------|--------|
| Version sorting | Natural sort of numbers in strings | 🔲 TODO |
| Extension sorting | Group by file extension | 🔲 TODO |
| Multiple sort keys | Sort by size then name | 🔲 TODO |

## Phase 3: User Experience

### 3.1 Configuration

| Feature | Description | Status |
|---------|-------------|--------|
| Configuration file | `~/.ekarc` | 🔲 TODO |
| Environment variables | `EKA_COLOR`, `EKA_SORT`, etc. | 🔲 TODO |
| Theme support | Multiple color schemes | 🔲 TODO |

### 3.2 Interactive Features

| Feature | Description | Status |
|---------|-------------|--------|
| Directory navigation | cd, pushd/popd support | 🔲 TODO |
| File selection | Interactive file selection | 🔲 TODO |
| Quick preview | Preview file details | 🔲 TODO |

### 3.3 Aliases and Shortcuts

| Feature | Description | Status |
|---------|-------------|--------|
| Built-in aliases | la=eka -a, l=eka -l, etc. | 🔲 TODO |
| Custom aliases | User-defined shortcuts | 🔲 TODO |

## Phase 4: Platform Integration

### 4.1 System Integration

| Feature | Description | Status |
|---------|-------------|--------|
| macOS support | Native macOS integration | 🔲 TODO |
| Linux support | Full Linux support | 🔲 TODO |
| Windows support | Windows compatibility | 🔲 TODO |
| BSD support | FreeBSD/OpenBSD support | 🔲 TODO |

### 4.2 Terminal Integration

| Feature | Description | Status |
|---------|-------------|--------|
| True color support | 24-bit color support | 🔲 TODO |
| Terminal detection | Auto-detect terminal capabilities | 🔲 TODO |
| Mouse support | Click to navigate | 🔲 TODO |
| Unicode support | Full Unicode handling | 🔲 TODO |

## Phase 5: Testing and Documentation

### 5.1 Testing

| Feature | Description | Status |
|---------|-------------|--------|
| Unit tests | Core functionality tests | 🔲 TODO |
| Integration tests | End-to-end testing | 🔲 TODO |
| Property-based tests | Fuzz testing | 🔲 TODO |
| Cross-platform CI | CI/CD pipeline | 🔲 TODO |
| Benchmarking | Performance benchmarks | 🔲 TODO |

### 5.2 Documentation

| Feature | Description | Status |
|---------|-------------|--------|
| Man page | Unix man page | 🔲 TODO |
| Built-in help | `--help` enhancement | 🔲 TODO |
| Examples | Usage examples | 🔲 TODO |
| FAQ | Frequently asked questions | 🔲 TODO |

## Implementation Priorities

### Immediate (v0.2.0)
1. Add missing basic flags (-1, -F, -R, -S, -t)
2. Fix sorting by size/time
3. Add inode display
4. Fix symlink handling

### Short-term (v0.3.0)
1. Configuration file support
2. Extended attributes
3. Performance optimization

### Medium-term (v0.4.0)
1. Interactive mode
2. Theme support
3. Full cross-platform support

### Long-term (v1.0.0)
1. Complete feature parity with GNU ls
2. Stability and testing
3. Release production version

## Comparison with GNU ls

| Feature | GNU ls | Eka Target |
|---------|--------|------------|
| Basic listing | ✅ | ✅ (Complete) |
| Color support | ✅ (with LS_COLORS) | ✅ (Complete) |
| Long format | ✅ | ✅ (Complete) |
| All flags | ✅ 60+ | 🔲 ~10 |
| Recursive | ✅ | 🔲 TODO |
| Sorting options | ✅ 8 types | 🔲 2 types |
| Configuration | Environment vars | 🔲 TODO |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## References

- [GNU Coreutils ls](https://www.gnu.org/software/coreutils/manual/html_node/ls-invocation.html)
- [POSIX ls specification](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/ls.html)
