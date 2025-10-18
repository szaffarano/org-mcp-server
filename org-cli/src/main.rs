use anyhow::Result;
use clap::{Parser, Subcommand};
use org_core::OrgMode;

mod commands;
mod config;
use commands::{
    ConfigCommand, ElementByIdCommand, HeadingCommand, InitCommand, ListCommand, OutlineCommand,
    ReadCommand, SearchCommand,
};
use config::CliAppConfig;

#[derive(Parser)]
#[command(name = "org")]
#[command(about = "A CLI tool for org-mode functionality")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Root directory containing org-mode files
    #[arg(short, long)]
    root_directory: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configuration management
    Config(ConfigCommand),
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
    /// Search for text content across all org files using fuzzy matching
    Search(SearchCommand),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Config(cmd) => cmd.execute(cli.config),
        _ => {
            // Load configuration with CLI overrides for non-config commands
            let config = CliAppConfig::load(
                cli.config,
                cli.root_directory,
                None, // log_level not needed for CLI
            )?;

            let org_mode = OrgMode::new(config.org)?;
            match cli.command {
                Commands::Config(_) => unreachable!(),
                Commands::List(cmd) => cmd.execute(org_mode, config.cli),
                Commands::Init(cmd) => cmd.execute(org_mode, config.cli),
                Commands::Read(cmd) => cmd.execute(org_mode, config.cli),
                Commands::Outline(cmd) => cmd.execute(org_mode, config.cli),
                Commands::Heading(cmd) => cmd.execute(org_mode, config.cli),
                Commands::ElementById(cmd) => cmd.execute(org_mode, config.cli),
                Commands::Search(cmd) => cmd.execute(org_mode, config.cli),
            }
        }
    }
}
