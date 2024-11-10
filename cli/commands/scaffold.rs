use std::fs;
use std::path::Path;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    scaffold: Scaffold,
}

#[derive(Subcommand, Debug)]
enum Scaffold {
    App { name: String },
}

const APP_FILES: &[(&str, &str)] = &[
    ("main.rs", include_str!("../templates/scaffold_app/main.rs")),
    (
        "build.rs",
        include_str!("../templates/scaffold_app/build.rs"),
    ),
    (
        "Cargo.toml",
        include_str!("../templates/scaffold_app/Cargo.toml"),
    ),
];
fn scaffold_app(name: &str) -> Result<()> {
    let apps_dir = Path::new("apps");
    if !apps_dir.exists() {
        bail!(
            "missing apps dir at: {}",
            apps_dir.canonicalize().unwrap().display()
        );
    }

    let app_dir = apps_dir.join(name);

    for (dest, template) in APP_FILES {
        let dest_path = app_dir.join(dest);

        let contents = template.replace("<NAME>", name);

        progress!("GEN", dest_path.display());
        fs::write(&dest_path, contents)
            .with_context(|| format!("failed to write to {}", dest_path.display()))?;
    }

    Ok(())
}

pub fn main(args: Args) -> Result<()> {
    match args.scaffold {
        Scaffold::App { name } => scaffold_app(&name)?,
    }

    Ok(())
}
