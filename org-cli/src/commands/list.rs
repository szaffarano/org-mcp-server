use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct ListCommand {
    /// Directory to search for org files
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

impl ListCommand {
    pub fn execute(&self) -> Result<()> {
        let org_mode = OrgMode::new(&self.dir)?;
        let files = org_mode.list_files()?;

        match self.format {
            OutputFormat::Plain => {
                if files.is_empty() {
                    println!("No .org files found in {}", self.dir);
                } else {
                    println!("Found {} .org files in {}:", files.len(), self.dir);
                    for file in files {
                        println!("  {file}");
                    }
                }
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "directory": self.dir,
                    "count": files.len(),
                    "files": files
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        }

        Ok(())
    }
}
