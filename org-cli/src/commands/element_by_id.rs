use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct ElementByIdCommand {
    /// The ID of the element to extract
    id: String,

    /// Directory containing org files
    #[arg(short, long, default_value = "~/org/")]
    dir: String,
}

impl ElementByIdCommand {
    pub fn execute(&self) -> Result<()> {
        let org_mode = OrgMode::new(&self.dir)?;
        let content = org_mode.get_element_by_id(&self.id)?;
        println!("{}", content);
        Ok(())
    }
}
