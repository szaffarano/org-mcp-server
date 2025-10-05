use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct ElementByIdCommand {
    /// The ID of the element to extract
    id: String,
}

impl ElementByIdCommand {
    pub fn execute(&self, org_mode: OrgMode) -> Result<()> {
        let content = org_mode.get_element_by_id(&self.id)?;
        println!("{}", content);
        Ok(())
    }
}
