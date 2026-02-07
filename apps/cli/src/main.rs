mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "bamboo")]
#[command(about = "A fast static site generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New {
        name: String,
    },
    Init,
    Build {
        #[arg(long, default_value = "default")]
        theme: String,

        #[arg(long, short)]
        input: Option<PathBuf>,

        #[arg(long, short, default_value = "dist")]
        output: PathBuf,

        #[arg(long)]
        drafts: bool,

        #[arg(long)]
        base_url: Option<String>,

        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        clean: bool,
    },
    Serve {
        #[arg(long, default_value = "default")]
        theme: String,

        #[arg(long, short)]
        input: Option<PathBuf>,

        #[arg(long, short, default_value = "dist")]
        output: PathBuf,

        #[arg(long)]
        drafts: bool,

        #[arg(long, default_value = "3000")]
        port: u16,

        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        clean: bool,

        #[arg(long)]
        open: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::New { name } => commands::new_site(&name),
        Commands::Init => commands::init_site(),
        Commands::Build {
            theme,
            input,
            output,
            drafts,
            base_url,
            clean,
        } => commands::build_site(
            &theme,
            input.as_deref(),
            &output,
            drafts,
            base_url.as_deref(),
            clean,
        ),
        Commands::Serve {
            theme,
            input,
            output,
            drafts,
            port,
            clean,
            open,
        } => {
            commands::serve_site(&theme, input.as_deref(), &output, drafts, port, clean, open).await
        }
    };

    if let Err(error) = result {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
