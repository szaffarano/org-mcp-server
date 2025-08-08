use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct OutlineCommand {
    /// Relative path to the org file to get outline from
    file: String,

    /// Directory containing org files
    #[arg(short, long, default_value = "~/org/")]
    dir: String,

    /// Output format
    #[arg(short, long, default_value = "plain")]
    format: OutputFormat,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

impl OutlineCommand {
    pub fn execute(&self) -> Result<()> {
        let org_mode = OrgMode::new(&self.dir)?;
        let tree = org_mode.get_outline(&self.file)?;

        match self.format {
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
