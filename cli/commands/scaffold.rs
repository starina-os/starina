use std::fs;
use std::fs::OpenOptions;
use std::path::Path;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use regex::Regex;
use starina_types::spec::AppSpec;
use starina_types::spec::Spec;
use starina_types::spec::SpecFile;

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
    if !Regex::new("^[a-z_][a-z0-9_]*$")?.is_match(name) {
        bail!(
            "invalid app name: \"{}\" (must match /^[a-z_][a-z0-9_]*$/)",
            name
        );
    }

    let apps_dir = Path::new("apps");
    if !apps_dir.exists() {
        bail!(
            "missing apps dir at: {}",
            apps_dir.canonicalize().unwrap().display()
        );
    }

    let app_dir = apps_dir.join(name);
    fs::create_dir_all(&app_dir)
        .with_context(|| format!("failed to mkdir: {}", app_dir.display()))?;

    for (dest, template) in APP_FILES {
        let dest_path = app_dir.join(dest);

        let contents = template.replace("<NAME>", name);

        progress!("GEN", dest_path.display());
        fs::write(&dest_path, contents)
            .with_context(|| format!("failed to write to {}", dest_path.display()))?;
    }

    let app_spec_path = app_dir.join("app.spec.json");
    let app_spec_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&app_spec_path)
        .with_context(|| format!("failed to open file: {}", app_spec_path.display()))?;
    serde_json::to_writer(
        app_spec_file,
        &SpecFile {
            name: name.to_string(),
            spec: Spec::App(AppSpec {
                depends: vec![],
                provides: vec![],
            }),
        },
    )
    .with_context(|| format!("failed to generate: {}", app_spec_path.display()))?;

    Ok(())
}

pub fn main(args: Args) -> Result<()> {
    match args.scaffold {
        Scaffold::App { name } => scaffold_app(&name)?,
    }

    Ok(())
}
