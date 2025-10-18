use anyhow::Result;
use clap::Args;
use org_core::{OrgMode, config::CliConfig};

#[derive(Args)]
pub struct ElementByIdCommand {
    /// The ID of the element to extract
    id: String,
}

impl ElementByIdCommand {
    pub fn execute(&self, org_mode: OrgMode, _cli: CliConfig) -> Result<()> {
        let content = org_mode.get_element_by_id(&self.id)?;
        println!("{}", content);
        Ok(())
    }
}
