mod cli;
mod colors;
mod directory;
mod formatter;
mod types;

use clap::Parser;
use cli::Args;
use directory::{io_message, read_directory, read_directory_recursive};
use formatter::Formatter;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process;
use types::FileEntry;

/// Exit status for serious trouble (e.g. a command-line argument cannot be accessed)
const EXIT_SERIOUS: i32 = 2;

fn main() {
    let args = Args::parse();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let code = match run(&mut out, &args) {
        Ok(code) => code,
        // Closed pipe (e.g. `eka | head`) is not an error
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(e) => {
            eprintln!("eka: write error: {}", io_message(&e));
            1
        }
    };
    if let Err(e) = out.flush()
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        eprintln!("eka: write error: {}", io_message(&e));
    }
    process::exit(code);
}

fn print_entries(out: &mut impl Write, entries: &[FileEntry], args: &Args) -> io::Result<()> {
    if args.long {
        Formatter::format_long(out, entries, args)
    } else {
        Formatter::format_short(out, entries, args)
    }
}

/// Lists every command-line path. Returns the process exit status.
fn run(out: &mut impl Write, args: &Args) -> io::Result<i32> {
    let mut status = 0;
    let mut files: Vec<FileEntry> = Vec::new();
    let mut dirs: Vec<&str> = Vec::new();

    // Classify arguments: non-directories (and everything under -d) are listed
    // as themselves, first; directories are expanded afterwards.
    for path in &args.paths {
        let p = Path::new(path);
        // Follow a command-line symlink to a directory unless -d or -l asks for the link itself
        let follows = !args.directories && !args.long;
        let meta = if follows { fs::metadata(p) } else { fs::symlink_metadata(p) };
        let meta = meta.or_else(|_| fs::symlink_metadata(p));
        match meta {
            Ok(m) if m.is_dir() && !args.directories => dirs.push(path),
            Ok(_) => match FileEntry::with_name(p.to_path_buf(), p.as_os_str().to_owned()) {
                Ok(e) => files.push(e),
                Err(e) => {
                    eprintln!("eka: cannot access '{}': {}", path, io_message(&e));
                    status = EXIT_SERIOUS;
                }
            },
            Err(e) => {
                eprintln!("eka: cannot access '{}': {}", path, io_message(&e));
                status = EXIT_SERIOUS;
            }
        }
    }

    let show_names = args.paths.len() > 1 || args.recursive;
    let mut first = true;

    if !files.is_empty() {
        directory::sort_entries(&mut files, args);
        print_entries(out, &files, args)?;
        first = false;
    }

    let mut had_error = false;
    for path in dirs {
        let listing = if args.recursive {
            read_directory_recursive(Path::new(path), args, &mut had_error)
        } else {
            read_directory(Path::new(path), args, &mut had_error).map(|entries| {
                vec![directory::DirectoryEntry { path: path.to_string(), entries }]
            })
        };
        match listing {
            Ok(dirs) => {
                for d in dirs {
                    if !first {
                        out.write_all(b"\n")?;
                    }
                    first = false;
                    if show_names {
                        writeln!(out, "{}:", d.path)?;
                    }
                    if args.long {
                        writeln!(out, "total {}", d.entries.len())?;
                    }
                    print_entries(out, &d.entries, args)?;
                }
            }
            Err(e) => {
                eprintln!("eka: cannot open directory '{}': {}", path, io_message(&e));
                status = EXIT_SERIOUS;
            }
        }
    }

    if had_error && status == 0 {
        status = 1;
    }
    Ok(status)
}
