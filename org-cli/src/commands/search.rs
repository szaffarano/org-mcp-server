use crate::config::CliConfig;
use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct SearchCommand {
    /// Search query
    query: String,

    /// Maximum number of results to return
    #[arg(short, long)]
    limit: Option<usize>,

    /// Output format
    #[arg(short = 'f', long)]
    format: Option<OutputFormat>,

    /// Maximum snippet size in characters
    #[arg(short = 's', long, default_value = "100")]
    snippet_size: usize,

    /// Filter by tags (comma-separated)
    #[arg(short = 't', long, value_delimiter = ',')]
    tags: Option<Vec<String>>,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

impl SearchCommand {
    pub fn execute(&self, org_mode: OrgMode, cli: CliConfig) -> Result<()> {
        let results = if let Some(ref tags) = self.tags {
            org_mode.search_with_tags(
                &self.query,
                Some(tags.as_slice()),
                self.limit,
                Some(self.snippet_size),
            )?
        } else {
            org_mode.search(&self.query, self.limit, Some(self.snippet_size))?
        };

        let format = self.format.as_ref().unwrap_or({
            match cli.default_format.as_str() {
                "json" => &OutputFormat::Json,
                _ => &OutputFormat::Plain,
            }
        });

        match format {
            OutputFormat::Plain => {
                if results.is_empty() {
                    println!(
                        "No results found for query '{}' in {}",
                        self.query,
                        org_mode.config().org_directory
                    );
                } else {
                    println!(
                        "Found {} results for query '{}' in {}:",
                        results.len(),
                        self.query,
                        org_mode.config().org_directory
                    );
                    for result in results {
                        println!(
                            "{}: {} (score: {})",
                            result.file_path, result.snippet, result.score
                        );
                    }
                }
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "directory": org_mode.config().org_directory,
                    "query": self.query,
                    "count": results.len(),
                    "results": results
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        }

        Ok(())
    }
}
