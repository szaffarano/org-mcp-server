use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct SearchCommand {
    /// Search query
    query: String,

    /// Directory to search for org files
    #[arg(short, long, default_value = "~/org/")]
    dir: String,

    /// Maximum number of results to return
    #[arg(short, long)]
    limit: Option<usize>,

    /// Output format
    #[arg(short = 'f', long, default_value = "plain")]
    format: OutputFormat,

    /// Maximum snippet size in characters
    #[arg(short = 's', long, default_value = "100")]
    snippet_size: usize,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

impl SearchCommand {
    pub fn execute(&self) -> Result<()> {
        let org_mode = OrgMode::new(&self.dir)?;
        let results = org_mode.search(&self.query, self.limit, Some(self.snippet_size))?;

        match self.format {
            OutputFormat::Plain => {
                if results.is_empty() {
                    println!(
                        "No results found for query '{}' in {}",
                        self.query, self.dir
                    );
                } else {
                    println!(
                        "Found {} results for query '{}' in {}:",
                        results.len(),
                        self.query,
                        self.dir
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
                    "directory": self.dir,
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
