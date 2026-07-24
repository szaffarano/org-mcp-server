use crate::config::CliConfig;
use anyhow::{Result, anyhow};
use clap::Args;
use org_core::{ClearField, OrgMode, UpdateEntry};

#[derive(Args)]
pub struct UpdateTodoCommand {
    /// Org ID property of the target heading (wins over --file/--heading)
    #[arg(long)]
    id: Option<String>,

    /// File path relative to org directory (requires --heading)
    #[arg(short = 'F', long, requires = "heading")]
    file: Option<String>,

    /// Slash-separated heading path (e.g., 'Projects/Work'; requires --file)
    #[arg(long, requires = "file")]
    heading: Option<String>,

    /// New TODO state keyword (must match a configured keyword)
    #[arg(short = 's', long)]
    todo_state: Option<String>,

    /// New priority level: A, B, or C
    #[arg(short = 'p', long)]
    priority: Option<String>,

    /// Replace tags (comma-separated). Each tag must match [A-Za-z0-9_@]+
    #[arg(short = 't', long, value_delimiter = ',')]
    tags: Option<Vec<String>>,

    /// New SCHEDULED timestamp (ISO YYYY-MM-DD[ HH:MM] [repeater] [warning])
    #[arg(long)]
    scheduled: Option<String>,

    /// New DEADLINE timestamp (same format as --scheduled)
    #[arg(long)]
    deadline: Option<String>,

    /// CLOSED inactive timestamp (usually auto-managed on state transitions)
    #[arg(long)]
    closed: Option<String>,

    /// Fields to remove: todo-state,priority,tags,scheduled,deadline,closed
    #[arg(long, value_delimiter = ',')]
    clear: Vec<String>,

    /// Output format
    #[arg(short = 'f', long)]
    format: Option<OutputFormat>,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

impl UpdateTodoCommand {
    pub fn execute(&self, org_mode: OrgMode, cli: CliConfig) -> Result<()> {
        let clear = self
            .clear
            .iter()
            .map(|s| {
                s.parse::<ClearField>()
                    .map_err(|e| anyhow!("invalid --clear value: {e}"))
            })
            .collect::<Result<Vec<_>>>()?;

        let entry = UpdateEntry {
            id: self.id.clone(),
            file: self.file.clone(),
            heading_path: self.heading.clone(),
            todo_state: self.todo_state.clone(),
            priority: self.priority.clone(),
            tags: self.tags.clone(),
            scheduled: self.scheduled.clone(),
            deadline: self.deadline.clone(),
            closed: self.closed.clone(),
            clear,
        };

        let result = org_mode.update_todo(entry)?;

        let format = self.format.as_ref().unwrap_or({
            match cli.default_format.as_str() {
                "json" => &OutputFormat::Json,
                _ => &OutputFormat::Plain,
            }
        });

        match format {
            OutputFormat::Plain => {
                println!("Updated {}", result.file_path);
                println!("  {}", result.heading_line);
                for change in &result.changes {
                    println!("  {change}");
                }
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}
