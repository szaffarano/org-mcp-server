use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct ReadCommand {
    /// Relative path to the org file to read
    file: String,
}

impl ReadCommand {
    pub fn execute(&self, org_mode: OrgMode) -> Result<()> {
        let content = org_mode.read_file(&self.file)?;
        println!("{}", content);
        Ok(())
    }
}
