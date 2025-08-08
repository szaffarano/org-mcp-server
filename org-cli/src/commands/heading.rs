use anyhow::Result;
use clap::Args;
use org_core::OrgMode;

#[derive(Args)]
pub struct HeadingCommand {
    /// Relative path to the org file
    file: String,

    /// The heading to extract (without the * prefix)
    heading: String,

    /// Directory containing org files
    #[arg(short, long, default_value = "~/org/")]
    dir: String,
}

impl HeadingCommand {
    pub fn execute(&self) -> Result<()> {
        let org_mode = OrgMode::new(&self.dir)?;
        let content = org_mode.get_heading(&self.file, &self.heading)?;
        println!("{}", content);
        Ok(())
    }
}
