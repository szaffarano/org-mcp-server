use crate::config::CliConfig;
use anyhow::Result;
use chrono::TimeZone;
use chrono::{Local, NaiveDate};
use clap::{Args, Subcommand};
use org_core::{OrgMode, Priority, org_mode::AgendaViewType};

#[derive(Args)]
pub struct AgendaCommand {
    #[command(subcommand)]
    subcommand: AgendaSubcommand,

    /// Output format
    #[arg(short = 'f', long, global = true)]
    format: Option<OutputFormat>,

    /// Maximum number of results to return
    #[arg(short, long, global = true)]
    limit: Option<usize>,
}

#[derive(Subcommand)]
enum AgendaSubcommand {
    /// List all tasks (TODO/DONE items)
    List {
        /// Filter by TODO states (comma-separated, e.g., TODO,DONE)
        #[arg(short = 's', long, value_delimiter = ',')]
        states: Option<Vec<String>>,

        /// Filter by tags (comma-separated)
        #[arg(short = 't', long, value_delimiter = ',')]
        tags: Option<Vec<String>>,

        /// Filter by priority (A, B, C)
        #[arg(short = 'p', long)]
        priority: Option<PriorityArg>,
    },

    /// Show today's tasks
    Today {
        /// Filter by tags (comma-separated)
        #[arg(short = 't', long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },

    /// Show this week's tasks
    Week {
        /// Filter by tags (comma-separated)
        #[arg(short = 't', long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },

    /// Show tasks in custom date range
    Range {
        /// Start date (ISO 8601 format: YYYY-MM-DD)
        #[arg(short = 's', long)]
        start: String,

        /// End date (ISO 8601 format: YYYY-MM-DD)
        #[arg(short = 'e', long)]
        end: String,

        /// Filter by tags (comma-separated)
        #[arg(short = 't', long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Plain,
    Json,
}

#[derive(clap::ValueEnum, Clone)]
enum PriorityArg {
    A,
    B,
    C,
}

impl From<PriorityArg> for Priority {
    fn from(arg: PriorityArg) -> Self {
        match arg {
            PriorityArg::A => Priority::A,
            PriorityArg::B => Priority::B,
            PriorityArg::C => Priority::C,
        }
    }
}

impl AgendaCommand {
    pub fn execute(&self, org_mode: OrgMode, cli: CliConfig) -> Result<()> {
        let format = self.format.as_ref().unwrap_or({
            match cli.default_format.as_str() {
                "json" => &OutputFormat::Json,
                _ => &OutputFormat::Plain,
            }
        });

        match &self.subcommand {
            AgendaSubcommand::List {
                states,
                tags,
                priority,
            } => {
                let tasks = org_mode.list_tasks(
                    states.as_ref().map(|v| v.as_slice()),
                    tags.as_ref().map(|v| v.as_slice()),
                    priority.clone().map(Into::into),
                    self.limit,
                )?;

                match format {
                    OutputFormat::Plain => {
                        if tasks.is_empty() {
                            println!("No tasks found in {}", org_mode.config().org_directory);
                        } else {
                            println!(
                                "Found {} task(s) in {}:",
                                tasks.len(),
                                org_mode.config().org_directory
                            );
                            for task in tasks {
                                let state_str = task
                                    .todo_state
                                    .as_ref()
                                    .map(|s| format!("{s:?}"))
                                    .unwrap_or_default();
                                let priority_str = task
                                    .priority
                                    .as_ref()
                                    .map(|p| format!("[#{p:?}]"))
                                    .unwrap_or_default();
                                println!(
                                    "  {}{} {} ({}:[{}])",
                                    state_str,
                                    if priority_str.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" {priority_str}")
                                    },
                                    task.heading,
                                    task.file_path,
                                    task.position
                                        .map(|p| format!("{}:{}", p.start, p.end))
                                        .unwrap_or_default()
                                );
                                if let Some(ref deadline) = task.deadline {
                                    println!("    DEADLINE: {deadline}");
                                }
                                if let Some(ref scheduled) = task.scheduled {
                                    println!("    SCHEDULED: {scheduled}");
                                }
                            }
                        }
                    }
                    OutputFormat::Json => {
                        let json = serde_json::json!({
                            "directory": org_mode.config().org_directory,
                            "count": tasks.len(),
                            "tasks": tasks
                        });
                        println!("{}", serde_json::to_string_pretty(&json)?);
                    }
                }
            }

            AgendaSubcommand::Today { tags } => {
                let view = org_mode.get_agenda_view(
                    AgendaViewType::Today,
                    None,
                    tags.as_ref().map(|v| v.as_slice()),
                )?;

                self.print_agenda_view(view, format, &org_mode)?;
            }

            AgendaSubcommand::Week { tags } => {
                let view = org_mode.get_agenda_view(
                    AgendaViewType::CurrentWeek,
                    None,
                    tags.as_ref().map(|v| v.as_slice()),
                )?;

                self.print_agenda_view(view, format, &org_mode)?;
            }

            AgendaSubcommand::Range { start, end, tags } => {
                let from = NaiveDate::parse_from_str(start, "%Y-%m-%d")
                    .map_err(|e| anyhow::anyhow!("Failed to parse start date '{}': {}", start, e))?
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Invalid time components for date '{}'", start)
                    })?;

                let from = Local.from_local_datetime(&from).single().ok_or_else(|| {
                    anyhow::anyhow!("Failed to convert start date '{}' to local timezone", start)
                })?;

                let to = NaiveDate::parse_from_str(end, "%Y-%m-%d")
                    .map_err(|e| anyhow::anyhow!("Failed to parse end date '{}': {}", end, e))?
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| anyhow::anyhow!("Invalid time components for date '{}'", end))?;

                let to = Local.from_local_datetime(&to).single().ok_or_else(|| {
                    anyhow::anyhow!("Failed to convert end date '{}' to local timezone", end)
                })?;

                let view = org_mode.get_agenda_view(
                    AgendaViewType::Custom { from, to },
                    None,
                    tags.as_ref().map(|v| v.as_slice()),
                )?;

                self.print_agenda_view(view, format, &org_mode)?;
            }
        }

        Ok(())
    }

    fn print_agenda_view(
        &self,
        view: org_core::AgendaView,
        format: &OutputFormat,
        org_mode: &OrgMode,
    ) -> Result<()> {
        match format {
            OutputFormat::Plain => {
                let date_range =
                    if let (Some(start), Some(end)) = (&view.start_date, &view.end_date) {
                        format!(" ({start} to {end})")
                    } else {
                        String::new()
                    };

                if view.items.is_empty() {
                    println!(
                        "No scheduled tasks found{date_range} in {}",
                        org_mode.config().org_directory
                    );
                } else {
                    println!("Agenda{date_range} - {} task(s):", view.items.len());
                    for task in view.items {
                        let state_str = task
                            .todo_state
                            .as_ref()
                            .map(|s| format!("{s:?}"))
                            .unwrap_or_default();
                        let priority_str = task
                            .priority
                            .as_ref()
                            .map(|p| format!("[#{p:?}]"))
                            .unwrap_or_default();

                        let date_info = match (&task.scheduled, &task.deadline) {
                            (Some(s), Some(d)) if s == d => format!("SCHEDULED+DEADLINE: {s}"),
                            (Some(s), Some(d)) => format!("SCHEDULED: {s}, DEADLINE: {d}"),
                            (Some(s), None) => format!("SCHEDULED: {s}"),
                            (None, Some(d)) => format!("DEADLINE: {d}"),
                            (None, None) => String::new(),
                        };

                        println!(
                            "  {}{} {} ({})",
                            state_str,
                            if priority_str.is_empty() {
                                String::new()
                            } else {
                                format!(" {priority_str}")
                            },
                            task.heading,
                            task.file_path
                        );
                        if !date_info.is_empty() {
                            println!("    {date_info}");
                        }
                    }
                }
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "directory": org_mode.config().org_directory,
                    "start_date": view.start_date,
                    "end_date": view.end_date,
                    "count": view.items.len(),
                    "items": view.items
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        }

        Ok(())
    }
}
