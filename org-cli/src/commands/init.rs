use anyhow::Result;
use clap::Args;
use org_core::OrgMode;
use std::path::Path;

#[derive(Args)]
pub struct InitCommand {
    /// Directory to initialize as org directory
    path: Option<String>,
}

impl InitCommand {
    pub fn execute(&self) -> Result<()> {
        let dir = self.path.as_deref().unwrap_or("~/org/");

        match OrgMode::new(dir) {
            Ok(_) => {
                println!("✓ Org directory '{dir}' is valid and accessible");
            }
            Err(e) => {
                if let Some(expanded) = shellexpand::tilde(dir).as_ref().into() {
                    let path = Path::new(expanded);
                    if !path.exists() {
                        println!("Directory '{dir}' doesn't exist. Creating...");
                        std::fs::create_dir_all(path)?;
                        println!("✓ Created org directory '{dir}'");

                        OrgMode::new(dir)?;
                        println!("✓ Org directory '{dir}' is ready for use");
                    } else {
                        return Err(anyhow::anyhow!("Failed to initialize org directory: {e}"));
                    }
                } else {
                    return Err(anyhow::anyhow!("Failed to expand path: {dir}"));
                }
            }
        }

        Ok(())
    }
}
