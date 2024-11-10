use std::env::current_dir;
use std::env::set_current_dir;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;

fn move_to_project_dir() -> Result<()> {
    // Look for the directory with Makefile.
    let mut dir = current_dir()?;
    loop {
        if dir.join("Makefile").exists() && dir.join("Cargo.lock").exists() {
            set_current_dir(&dir)
                .with_context(|| format!("failed to chdir to {}", dir.display()))?;
            return Ok(());
        }

        if !dir.pop() {
            break;
        }
    }

    bail!("failed to locate the project directory");
}

fn ensure_buildconfig() -> Result<()> {
    let buildconfig_path = Path::new("buildconfig.mk");
    if !buildconfig_path.exists() {
        fs::write(
            buildconfig_path,
            include_bytes!("./templates/buildconfig.mk").as_ref(),
        )
        .context("failed to write buildconfig.mk")?;
    }

    Ok(())
}

pub fn run_make(cmd: &str) -> Result<()> {
    move_to_project_dir()?;
    ensure_buildconfig()?;
    let err = Command::new("make").arg(cmd).exec();
    panic!("failed to exec make: {}", err);
}
