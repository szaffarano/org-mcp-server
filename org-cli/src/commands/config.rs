use anyhow::Result;
use clap::{Args, Subcommand};
use org_core::Config;

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Initialize a default configuration file
    Init,
    /// Show current configuration
    Show,
    /// Show configuration file path
    Path,
}

impl ConfigCommand {
    pub fn execute(&self, config_file: Option<String>) -> Result<()> {
        match &self.action {
            ConfigAction::Init => self.init_config(config_file),
            ConfigAction::Show => self.show_config(config_file),
            ConfigAction::Path => self.show_path(config_file),
        }
    }

    fn init_config(&self, config_file: Option<String>) -> Result<()> {
        let config_path = if let Some(path) = config_file {
            std::path::PathBuf::from(path)
        } else {
            Config::default_config_path()?
        };

        if config_path.exists() {
            println!(
                "Configuration file already exists at: {}",
                config_path.display()
            );
            println!("Use 'org config show' to view current configuration");
            return Ok(());
        }

        let default_config = Config::default();
        default_config.save_to_file(&config_path)?;

        println!(
            "Default configuration file created at: {}",
            config_path.display()
        );
        println!("Edit this file to customize your org-mode setup");

        Ok(())
    }

    fn show_config(&self, config_file: Option<String>) -> Result<()> {
        let config = Config::load_with_overrides(config_file, None, None)
            .unwrap_or_else(|_| Config::default());
        let config_toml = toml::to_string_pretty(&config)?;
        println!("{}", config_toml);
        Ok(())
    }

    fn show_path(&self, config_file: Option<String>) -> Result<()> {
        let config_path = if let Some(path) = config_file {
            std::path::PathBuf::from(path)
        } else {
            Config::default_config_path()?
        };
        println!("{}", config_path.display());
        Ok(())
    }
}
