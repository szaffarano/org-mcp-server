use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct ListCommand {
    /// Output format
    #[arg(short, long)]
    format: Option<OutputFormat>,

    /// Filter by tags (comma-separated)
    #[arg(short = 't', long, value_delimiter = ',')]
    tags: Option<Vec<String>>,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

impl ListCommand {
    pub fn execute(&self, org_mode: OrgMode) -> Result<()> {
        let files = if let Some(ref tags) = self.tags {
            org_mode.list_files_by_tags(tags.as_slice())?
        } else {
            org_mode.list_files()?
        };

        let format = self.format.as_ref().unwrap_or({
            match org_mode.config().cli.default_format.as_str() {
                "json" => &OutputFormat::Json,
                _ => &OutputFormat::Plain,
            }
        });

        match format {
            OutputFormat::Plain => {
                if files.is_empty() {
                    println!(
                        "No .org files found in {}",
                        org_mode.config().org.org_directory
                    );
                } else {
                    println!(
                        "Found {} .org files in {}:",
                        files.len(),
                        org_mode.config().org.org_directory
                    );
                    for file in files {
                        println!("  {file}");
                    }
                }
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "directory": org_mode.config().org.org_directory,
                    "count": files.len(),
                    "files": files
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        }

        Ok(())
    }
}
