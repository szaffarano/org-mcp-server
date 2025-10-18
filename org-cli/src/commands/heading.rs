use anyhow::Result;
use clap::Args;
use org_core::{OrgMode, config::CliConfig};

#[derive(Args)]
pub struct HeadingCommand {
    /// Relative path to the org file
    file: String,

    /// The heading to extract (without the * prefix)
    heading: String,
}

impl HeadingCommand {
    pub fn execute(&self, org_mode: OrgMode, _cli: CliConfig) -> Result<()> {
        let content = org_mode.get_heading(&self.file, &self.heading)?;
        println!("{}", content);
        Ok(())
    }
}
