use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct ReadCommand {
    /// Relative path to the org file to read
    file: String,

    /// Directory containing org files
    #[arg(short, long, default_value = "~/org/")]
    dir: String,
}

impl ReadCommand {
    pub fn execute(&self) -> Result<()> {
        let org_mode = OrgMode::new(&self.dir)?;
        let content = org_mode.read_file(&self.file)?;
        println!("{}", content);
        Ok(())
    }
}
