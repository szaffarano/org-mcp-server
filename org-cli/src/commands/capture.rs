use crate::config::CliConfig;
use anyhow::{Result, anyhow};
use clap::Args;
use org_core::{CaptureEntry, OrgMode, PropertyPair};

#[derive(Args)]
pub struct CaptureCommand {
    /// Title for the new heading (required, non-empty, no newlines)
    title: String,

    /// Heading level (1..=19). Auto-determined if omitted
    #[arg(long)]
    level: Option<usize>,

    /// TODO state keyword (must match a configured keyword)
    #[arg(short = 's', long)]
    todo_state: Option<String>,

    /// Tags (comma-separated). Each tag must match [A-Za-z0-9_@]+
    #[arg(short = 't', long, value_delimiter = ',')]
    tags: Option<Vec<String>>,

    /// Priority level: A, B, or C
    #[arg(short = 'p', long)]
    priority: Option<String>,

    /// Body content to add below the heading
    #[arg(short = 'b', long)]
    body: Option<String>,

    /// File path relative to org directory (uses default notes file if omitted)
    #[arg(short = 'F', long)]
    file: Option<String>,

    /// Slash-separated heading path to insert under (e.g., 'Projects/Work')
    #[arg(long)]
    target_heading: Option<String>,

    /// SCHEDULED timestamp (ISO YYYY-MM-DD[ HH:MM] [repeater] [warning])
    #[arg(long)]
    scheduled: Option<String>,

    /// DEADLINE timestamp (same format as --scheduled)
    #[arg(long)]
    deadline: Option<String>,

    /// CLOSED inactive timestamp (same format as --scheduled)
    #[arg(long)]
    closed: Option<String>,

    /// Property drawer entry KEY=VALUE; repeatable.
    #[arg(long = "property", value_name = "KEY=VALUE")]
    properties: Vec<String>,

    /// Place capture under a Year/Month/Day datetree (use --datetree-date to override the day).
    #[arg(long)]
    datetree: bool,

    /// Override datetree day (YYYY-MM-DD). Implies --datetree.
    #[arg(long, value_name = "YYYY-MM-DD")]
    datetree_date: Option<String>,

    /// Output format
    #[arg(short = 'f', long)]
    format: Option<OutputFormat>,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

fn parse_property(arg: &str) -> Result<PropertyPair> {
    let (key, value) = arg
        .split_once('=')
        .ok_or_else(|| anyhow!("property must be KEY=VALUE: '{arg}'"))?;
    Ok(PropertyPair {
        key: key.to_string(),
        value: value.to_string(),
    })
}

impl CaptureCommand {
    pub fn execute(&self, org_mode: OrgMode, cli: CliConfig) -> Result<()> {
        let properties = if self.properties.is_empty() {
            None
        } else {
            Some(
                self.properties
                    .iter()
                    .map(|s| parse_property(s))
                    .collect::<Result<Vec<_>>>()?,
            )
        };

        // --datetree-date implies --datetree
        let datetree = self.datetree || self.datetree_date.is_some();

        let entry = CaptureEntry {
            title: self.title.clone(),
            level: self.level,
            todo_state: self.todo_state.clone(),
            tags: self.tags.clone(),
            priority: self.priority.clone(),
            body: self.body.clone(),
            file: self.file.clone(),
            target_heading: self.target_heading.clone(),
            scheduled: self.scheduled.clone(),
            deadline: self.deadline.clone(),
            closed: self.closed.clone(),
            properties,
            datetree,
            datetree_date: self.datetree_date.clone(),
        };

        let result = org_mode.capture_append(entry)?;

        let format = self.format.as_ref().unwrap_or({
            match cli.default_format.as_str() {
                "json" => &OutputFormat::Json,
                _ => &OutputFormat::Plain,
            }
        });

        match format {
            OutputFormat::Plain => {
                println!("Captured to {}", result.file_path);
                println!("  {}", result.heading_line);
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}
