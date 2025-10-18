use anyhow::Result;
use clap::Args;
use org_core::{OrgMode, config::CliConfig};

#[derive(Args)]
pub struct OutlineCommand {
    /// Relative path to the org file to get outline from
    file: String,

    /// Output format
    #[arg(short, long)]
    format: Option<OutputFormat>,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

impl OutlineCommand {
    pub fn execute(&self, org_mode: OrgMode, cli: CliConfig) -> Result<()> {
        let tree = org_mode.get_outline(&self.file)?;

        let format = self.format.as_ref().unwrap_or({
            match cli.default_format.as_str() {
                "json" => &OutputFormat::Json,
                _ => &OutputFormat::Plain,
            }
        });

        match format {
            OutputFormat::Plain => {
                if tree.children.is_empty() {
                    println!("No headings found in {}", self.file);
                } else {
                    let outline_text = tree.to_indented_string(0);
                    println!("{}", outline_text.trim_end());
                }
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&tree)?);
            }
        }

        Ok(())
    }
}
