# Eka - Modern `ls` Replacement

Eka is a modern replacement for the traditional `ls` command, written in Rust. It provides enhanced directory listing with color-coded file types and user-friendly output formats.

## Features

- **Color-coded output**: Different colors for directories, executables, symlinks, images, videos, audio, and compressed files
- **Multiple display formats**: Short and long listing formats
- **Flexible filtering**: Show/hide hidden files, list directories only
- **Human-readable sizes**: Display file sizes in KB, MB, GB format
- **Fast and lightweight**: Built with Rust for optimal performance

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/eka.git
cd eka

# Build the project
cargo build --release

# Install (optional)
cargo install --path .
```

## Usage

### Basic Commands

```bash
# List current directory (short format with colors)
eka

# List with long format
eka -l

# List only directories
eka -d

# Show hidden files
eka -a

# Long format with human-readable sizes
eka -lh

# List specific directory
eka /usr/local/bin

# Multiple paths
eka src/ tests/
```

### Command-Line Options

| Flag | Description |
|------|-------------|
| `-l`, `--long` | Long format with columns (permissions, size, date, name) |
| `-d`, `--directories` | List directories only |
| `-a`, `--all` | Show hidden files (starting with `.`) |
| `-H`, `--human-readable` | Display sizes in human-readable format (KB, MB, GB) |

## Color Scheme

Eka uses a distinctive color palette for easy file type identification:

| File Type | Color |
|-----------|-------|
| Directory | Cyan |
| Executable | Bright Green |
| Symlink | Yellow |
| Image | Magenta |
| Video | Red |
| Audio | Orange |
| Compressed | Purple |
| Hidden File | Dark Gray |
| Regular File | White |

## Project Structure

```
eka/
├── Cargo.toml
├── README.md
├── LICENSE
├── docs/
│   └── eka-architecture.md
└── src/
    ├── main.rs           # Application entry point
    ├── cli.rs            # CLI argument parsing
    ├── directory.rs      # Directory reading and processing
    ├── formatter.rs      # Output formatting
    ├── colors.rs         # ANSI color system
    └── types.rs          # Data type definitions
```

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

### Requirements

- Rust 1.70 or later
- Cargo (included with Rust)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
