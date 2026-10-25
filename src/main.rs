#![forbid(unsafe_code)]

mod cli;
mod collate;
mod colors;
mod directory;
mod formatter;
mod glob;
mod layout;
mod quoting;
mod types;

use clap::Parser;
use cli::Args;
use directory::{io_message, read_directory, read_directory_recursive, shown};
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

fn print_entries(
    out: &mut impl Write,
    fmt: &Formatter,
    entries: &[FileEntry],
    args: &Args,
) -> io::Result<()> {
    if args.is_long() {
        fmt.format_long(out, entries)
    } else {
        fmt.format_short(out, entries)
    }
}

/// Lists every command-line path. Returns the process exit status.
fn run(out: &mut impl Write, args: &Args) -> io::Result<i32> {
    let mut status = 0;
    let fmt = Formatter::new(args);
    let mut files: Vec<FileEntry> = Vec::new();
    let mut dirs: Vec<FileEntry> = Vec::new();

    // Classify arguments: non-directories (and everything under -d) are listed
    // as themselves, first; directories are expanded afterwards.
    for path in &args.paths {
        let p = Path::new(path);
        // A command-line symlink to a directory is followed unless -d or -l asks
        // for the link itself; -H and -L always follow
        let follows = args.dereference_cmdline() || (!args.directories && !args.is_long());
        let meta = if follows {
            fs::metadata(p)
        } else {
            fs::symlink_metadata(p)
        };
        let meta = meta.or_else(|_| fs::symlink_metadata(p));
        match meta {
            // Directory operands become entries too, so they sort like any other
            Ok(m) if m.is_dir() && !args.directories => {
                match FileEntry::with_name(p.to_path_buf(), p.as_os_str().to_owned(), follows) {
                    Ok(e) => dirs.push(e),
                    Err(e) => {
                        eprintln!("eka: cannot access {}: {}", shown(p), io_message(&e));
                        status = EXIT_SERIOUS;
                    }
                }
            }
            Ok(_) => match FileEntry::with_name(
                p.to_path_buf(),
                p.as_os_str().to_owned(),
                args.dereference_cmdline(),
            ) {
                Ok(e) => files.push(e),
                Err(e) => {
                    eprintln!("eka: cannot access {}: {}", shown(p), io_message(&e));
                    status = EXIT_SERIOUS;
                }
            },
            Err(e) => {
                eprintln!("eka: cannot access {}: {}", shown(p), io_message(&e));
                status = EXIT_SERIOUS;
            }
        }
    }

    let show_names = args.paths.len() > 1 || args.recursive;
    let mut first = true;

    if !files.is_empty() {
        directory::sort_entries_with(&mut files, args, collate::enabled());
        print_entries(out, &fmt, &files, args)?;
        first = false;
    }

    // ls orders the directory operands with the same options as everything else
    directory::sort_entries_with(&mut dirs, args, collate::enabled());

    let mut had_error = false;
    for operand in &dirs {
        let path = operand.path.as_path();
        let listing = if args.recursive {
            read_directory_recursive(path, args, &mut had_error)
        } else {
            read_directory(path, args, &mut had_error).map(|entries| {
                vec![directory::DirectoryEntry {
                    path: path.to_path_buf(),
                    entries,
                }]
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
                        fmt.write_heading(out, &d.path)?;
                    }
                    if args.is_long() || args.size {
                        fmt.write_total(out, &d.entries)?;
                    }
                    print_entries(out, &fmt, &d.entries, args)?;
                }
            }
            Err(e) => {
                eprintln!(
                    "eka: cannot open directory {}: {}",
                    shown(path),
                    io_message(&e)
                );
                status = EXIT_SERIOUS;
            }
        }
    }

    if had_error && status == 0 {
        status = 1;
    }
    Ok(status)
}
