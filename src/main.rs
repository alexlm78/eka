mod cli;
mod colors;
mod directory;
mod formatter;
mod types;

use clap::Parser;
use cli::Args;
use formatter::Formatter;
use std::process;

/// Main entry point for the eka file listing utility
fn main() {
    // Parse CLI arguments
    let args = Args::parse();

    // Process each provided path
    let paths = &args.paths;
    let num_paths = paths.len();

    for (index, path) in paths.iter().enumerate() {
        // Read directory contents
        match directory::read_directory(path, &args) {
            Ok(entries) => {
                // Display directory name when multiple paths are provided
                if num_paths > 1 {
                    if index > 0 {
                        println!();
                    }
                    println!("{}:", path);
                } else if args.long {
                    // In long format, display total block count
                    Formatter::print_header(path, entries.len());
                }

                // Format and display entries based on display mode
                if args.long {
                    Formatter::format_long(&entries, args.human_readable);
                } else {
                    Formatter::format_short(&entries);
                }
            }
            Err(e) => {
                eprintln!("eka: {}", e);
                // Exit with error code if single path fails
                if num_paths == 1 {
                    process::exit(1);
                }
            }
        }
    }
}
