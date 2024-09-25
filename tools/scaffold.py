#!/usr/bin/env python3
import argparse
import sys
import json
import re
from pathlib import Path

CARGO_TOML = """\
[package]
name = "<NAME>"
version = { workspace = true }
authors = { workspace = true }
edition = { workspace = true }

[[bin]]
name = "<NAME>"
path = "main.rs"

[dependencies]
ftl_api = { workspace = true }

[build-dependencies]
ftl_autogen = { workspace = true }
"""

MAIN_RS = """\
#![no_std]
#![no_main]

use ftl_api::environ::Environ;
use ftl_api::prelude::*;

ftl_api::autogen!();

#[no_mangle]
pub fn main(mut env: Environ) {
    info!("Hello World!");
}
"""

BUILD_RS = """\
fn main() {
    ftl_autogen::generate_for_app().expect("autogen failed");
}
"""

def progress(msg):
    print(f"\033[1;94m==>\033[0m\033[1m {msg}\033[0m")

def generate(path: Path, content: str):
    print(f"  GEN {path}")
    if path.exists():
        error(f"File '{path}' already exists")

    Path(path.parent).mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        f.write(content)


def error(msg):
    print(f"\033[1;91mError: {msg}\033[0m")
    sys.exit(1)

def generate_app(args):
    app_name = args.name
    app_dir = Path("apps") / app_name

    def replace_placeholders(content):
        content = content.replace("<NAME>", app_name)
        return content

    if re.match(r"^[a-z][a-z0-9_]*$", app_name) is None:
        error(f"Invalid app name '{app_name}' (must be lowercase alphanumeric with underscores)")

    generate(app_dir / "Cargo.toml", replace_placeholders(CARGO_TOML))
    generate(app_dir / "build.rs", replace_placeholders(BUILD_RS))
    generate(app_dir / "main.rs", replace_placeholders(MAIN_RS))
    generate(app_dir / "app.spec.json", json.dumps({
        "name": app_name,
        "kind": "app/v0",
        "spec": {
            "depends": [],
            "provides": []
        }
    }, indent=2))

    print()
    progress(f"generated app at {app_dir}")

def main():
    parser = argparse.ArgumentParser(description="Generate template code")
    parser.add_argument("--type", help="The artifact type", choices=["app"])
    parser.add_argument("name", help="The name of artifact to be generated")
    args = parser.parse_args()

    if args.type == "app":
        generate_app(args)
    else:
        error(f"Unknown artifact type '{args.type}'")

if __name__ == "__main__":
    main()
