mod cli;
mod colors;
mod directory;
mod formatter;
mod types;

use clap::Parser;
use cli::Args;
use directory::{read_directory, read_directory_recursive};
use formatter::Formatter;
use std::process;

/// Main entry point for the eka file listing utility
fn main() {
    // Parse CLI arguments
    let args = Args::parse();

    // Process each provided path
    let paths = &args.paths;
    let num_paths = paths.len();

    // Handle recursive listing
    if args.recursive {
        for (index, path) in paths.iter().enumerate() {
            match read_directory_recursive(path, &args) {
                Ok(directories) => {
                    // Display separator between multiple paths
                    if num_paths > 1 && index > 0 {
                        println!();
                    }

                    // Print each directory
                    for (dir_index, dir_entry) in directories.iter().enumerate() {
                        if directories.len() > 1 {
                            if dir_index > 0 || num_paths > 1 {
                                println!();
                            }
                            Formatter::print_header(&dir_entry.path, dir_entry.entries.len());
                        } else if args.long {
                            // In long format, display total block count
                            Formatter::print_header(&dir_entry.path, dir_entry.entries.len());
                        }

                        // Format and display entries based on display mode
                        if args.long {
                            Formatter::format_long(&dir_entry.entries, &args);
                        } else {
                            Formatter::format_short(&dir_entry.entries, &args);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("eka: {}", e);
                    if num_paths == 1 {
                        process::exit(1);
                    }
                }
            }
        }
        return;
    }

    // Non-recursive listing
    for (index, path) in paths.iter().enumerate() {
        // Read directory contents
        match read_directory(path, &args) {
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
                    Formatter::format_long(&entries, &args);
                } else {
                    Formatter::format_short(&entries, &args);
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
