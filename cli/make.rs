use std::fs;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;

pub fn ensure_buildconfig() -> Result<()> {
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
    ensure_buildconfig()?;

    let err = Command::new("make").arg(cmd).exec();
    panic!("failed to exec make: {}", err);
}
