use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
use commands::{
    ElementByIdCommand, HeadingCommand, InitCommand, ListCommand, OutlineCommand, ReadCommand,
};

#[derive(Parser)]
#[command(name = "org")]
#[command(about = "A CLI tool for org-mode functionality")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all .org files in a directory
    List(ListCommand),
    /// Initialize or validate an org directory
    Init(InitCommand),
    /// Read the contents of an org file
    Read(ReadCommand),
    /// Get the outline (headings) of an org file
    Outline(OutlineCommand),
    /// Extract content from a specific heading in an org file
    Heading(HeadingCommand),
    /// Extract content from an element by ID across all org files
    ElementById(ElementByIdCommand),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List(cmd) => cmd.execute(),
        Commands::Init(cmd) => cmd.execute(),
        Commands::Read(cmd) => cmd.execute(),
        Commands::Outline(cmd) => cmd.execute(),
        Commands::Heading(cmd) => cmd.execute(),
        Commands::ElementById(cmd) => cmd.execute(),
    }
}
