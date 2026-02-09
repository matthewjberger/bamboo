+++
title = "Command-Line Tools"
weight = 11
template = "book.html"
+++

# Command-Line Tools

Rust excels at building fast, reliable command-line tools. The `clap` crate handles argument parsing while the ecosystem provides crates for colors, progress bars, and interactive prompts.

## Argument Parsing with Clap

Define your CLI interface with clap's derive API:

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "greet", about = "A friendly greeting tool")]
struct Args {
    /// Name of the person to greet
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u32,

    /// Use uppercase greeting
    #[arg(short, long)]
    uppercase: bool,
}

fn main() {
    let args = Args::parse();

    for _ in 0..args.count {
        let greeting = format!("Hello, {}!", args.name);
        if args.uppercase {
            println!("{}", greeting.to_uppercase());
        } else {
            println!("{greeting}");
        }
    }
}
```

## Subcommands

Structure complex CLIs with subcommands:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tasks", about = "Task management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task
    Add {
        /// Task description
        description: String,

        /// Priority level (1-5)
        #[arg(short, long, default_value_t = 3)]
        priority: u8,
    },

    /// List all tasks
    List {
        /// Show only completed tasks
        #[arg(long)]
        done: bool,
    },

    /// Mark a task as complete
    Done {
        /// Task ID
        id: u32,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { description, priority } => {
            println!("Adding task (priority {priority}): {description}");
        }
        Commands::List { done } => {
            if done {
                println!("Showing completed tasks");
            } else {
                println!("Showing all tasks");
            }
        }
        Commands::Done { id } => {
            println!("Marking task {id} as complete");
        }
    }
}
```

## Colored Output

Use the `colored` crate for terminal colors:

```rust
use colored::Colorize;

fn print_status(passed: usize, failed: usize) {
    println!("{}", "Test Results".bold().underline());
    println!("  {} {}", "PASSED".green().bold(), passed);
    if failed > 0 {
        println!("  {} {}", "FAILED".red().bold(), failed);
    }
}

fn print_warning(message: &str) {
    eprintln!("{} {message}", "warning:".yellow().bold());
}
```

## Progress Bars

Show progress for long-running operations with `indicatif`:

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

fn process_files(files: &[String]) {
    let progress = ProgressBar::new(files.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for file in files {
        progress.set_message(file.clone());
        process_single_file(file);
        progress.inc(1);
    }

    progress.finish_with_message("Done!");
}
```

## Reading stdin

Process piped input:

```rust
use std::io::{self, BufRead};

fn count_lines() -> io::Result<()> {
    let stdin = io::stdin();
    let mut line_count = 0;
    let mut word_count = 0;

    for line in stdin.lock().lines() {
        let line = line?;
        line_count += 1;
        word_count += line.split_whitespace().count();
    }

    println!("{line_count} lines, {word_count} words");
    Ok(())
}
```

## Exit Codes

Communicate success or failure to the shell:

```rust
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    process(&config)?;
    Ok(())
}
```

## Summary

Use `clap` for argument parsing — its derive API is clean and generates help text automatically. Add `colored` for terminal output, `indicatif` for progress bars, and `dialoguer` for interactive prompts. Rust's small binary sizes and fast startup make it an ideal choice for CLI tools.
