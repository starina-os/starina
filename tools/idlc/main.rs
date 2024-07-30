use std::fmt;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use ftl_types::idl;
use ftl_types::idl::IdlFile;
use ftl_types::spec::DependWithName;
use ftl_types::spec::Spec;
use ftl_types::spec::SpecFile;
use minijinja::context;
use minijinja::Environment;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
struct Field {
    name: String,
    is_handle: bool,
    builder_ty: String,
    raw_ty: String,
}

#[derive(Debug, Serialize, Clone)]
struct Message {
    protocol_name: String,
    name: String,
    msgid: isize,
    num_handles: usize,
    fields: Vec<Field>,
}

#[derive(Debug, Serialize)]
struct UsedMessage {
    /// `"PingRequest"`, `"PingReply"`, ...
    camel_name: String,
    /// `ftl_api_autogen::protocols::ping::PingRequest`, ...
    ty: String,
}

#[derive(Debug, Serialize)]
struct Protocol {
    name: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct App {
    name: String,
    depends: Vec<DependWithName>,
    used_messages: Vec<UsedMessage>,
}

fn resolve_builder_type_name(ty: &idl::Ty) -> String {
    match ty {
        idl::Ty::Int32 => "i32".to_string(),
        idl::Ty::Handle => "ftl_types::handle::HandleId".to_string(),
        idl::Ty::Bytes { capacity } => format!("ftl_types::idl::BytesField<{capacity}>"),
        idl::Ty::String { capacity } => format!("ftl_types::idl::StringField<{capacity}>"),
    }
}

fn resolve_raw_type_name(ty: &idl::Ty) -> String {
    match ty {
        idl::Ty::Int32 => "i32".to_string(),
        idl::Ty::Handle => "ftl_types::handle::HandleId".to_string(),
        idl::Ty::Bytes { capacity } => format!("ftl_types::idl::BytesField<{capacity}>"),
        idl::Ty::String { capacity } => format!("ftl_types::idl::StringField<{capacity}>"),
    }
}

struct CamelCase<'a>(&'a str);

impl fmt::Display for CamelCase<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut next_upper = true;
        for c in self.0.chars() {
            if c == '_' {
                next_upper = true;
            } else if next_upper {
                write!(f, "{}", c.to_ascii_uppercase())?;
                next_upper = false;
            } else {
                write!(f, "{}", c)?;
            }
        }
        Ok(())
    }
}

fn visit_fields(idl_fields: &[idl::Field]) -> Vec<Field> {
    let mut fields = Vec::with_capacity(idl_fields.len());
    for f in idl_fields {
        fields.push(Field {
            name: f.name.clone(),
            is_handle: f.ty == idl::Ty::Handle,
            builder_ty: resolve_builder_type_name(&f.ty),
            raw_ty: resolve_raw_type_name(&f.ty),
        });
    }

    fields
}

fn run_rustfmt(file: &Path) -> Result<()> {
    let output = std::process::Command::new("rustup")
        .args(["run", "nightly", "rustfmt"])
        .arg(file)
        .output()
        .with_context(|| format!("failed to run rustfmt on {}", file.display()))?;

    if !output.status.success() {
        anyhow::bail!(
            "rustfmt failed with status {}: {}",
            output.status,
            std::str::from_utf8(&output.stderr).unwrap()
        );
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    autogen_outfile: PathBuf,
    #[arg(long)]
    api_autogen_outfile: PathBuf,
    #[arg(long)]
    idl_file: PathBuf,
    #[arg(long)]
    app_specs: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let idl_file = File::open(args.idl_file)?;
    let idl: IdlFile = serde_json::from_reader(&idl_file)?;

    let mut next_msgid = 1;
    let mut protocols = Vec::new();
    let mut all_messages = Vec::new();
    for protocol in idl.protocols {
        let mut messages = Vec::new();
        if let Some(oneways) = &protocol.oneways {
            for oneway in oneways {
                let req_fields = visit_fields(&oneway.fields);
                let msg = Message {
                    protocol_name: protocol.name.clone(),
                    name: format!("{}", CamelCase(&oneway.name)),
                    msgid: next_msgid,
                    num_handles: req_fields.iter().filter(|f| f.is_handle).count(),
                    fields: req_fields,
                };
                next_msgid += 1;

                all_messages.push(msg.clone());
                messages.push(msg);
            }
        }

        if let Some(rpcs) = &protocol.rpcs {
            for rpc in rpcs {
                let req_fields = visit_fields(&rpc.request.fields);
                let req_msg = Message {
                    protocol_name: protocol.name.clone(),
                    name: format!("{}Request", CamelCase(&rpc.name)),
                    msgid: next_msgid,
                    num_handles: req_fields.iter().filter(|f| f.is_handle).count(),
                    fields: req_fields,
                };
                next_msgid += 1;

                let res_fields = visit_fields(&rpc.response.fields);
                let res_msg = Message {
                    protocol_name: protocol.name.clone(),
                    name: format!("{}Reply", CamelCase(&rpc.name)),
                    msgid: next_msgid,
                    num_handles: res_fields.iter().filter(|f| f.is_handle).count(),
                    fields: res_fields,
                };
                next_msgid += 1;

                all_messages.push(req_msg.clone());
                all_messages.push(res_msg.clone());
                messages.push(req_msg);
                messages.push(res_msg);
            }
        }

        protocols.push(Protocol {
            name: protocol.name.clone(),
            messages,
        });
    }

    let mut apps = Vec::new();
    for spec_path in args.app_specs {
        let spec_file = File::open(&spec_path)
            .with_context(|| format!("failed to open {}", spec_path.display()))?;
        let spec: SpecFile = serde_json::from_reader(&spec_file)
            .with_context(|| format!("failed to parse {}", spec_path.display()))?;

        let mut used_messages = Vec::new();
        for m in &all_messages {
            used_messages.push(UsedMessage {
                camel_name: format!("{}", CamelCase(&m.name)),
                ty: format!(
                    "ftl_autogen::protocols::{}::{}",
                    &m.protocol_name,
                    CamelCase(&m.name)
                ),
            });
        }

        match spec.spec {
            Spec::App(app) => {
                apps.push(App {
                    name: spec.name,
                    depends: app.depends,
                    used_messages,
                });
            }
            _ => {
                anyhow::bail!("unexpected spec type for {}", spec_path.display());
            }
        }
    }

    let mut j2env = Environment::new();
    j2env
        .add_template("ftl_autogen", include_str!("templates/ftl_autogen.rs.j2"))
        .unwrap();
    j2env
        .add_template(
            "ftl_api_autogen",
            include_str!("templates/ftl_api_autogen.rs.j2"),
        )
        .unwrap();

    let template = j2env.get_template("ftl_autogen")?;
    let lib_rs = template.render(context! {
        protocols => protocols,
    })?;
    std::fs::write(&args.autogen_outfile, lib_rs)?;
    run_rustfmt(&args.autogen_outfile)?;

    let template = j2env.get_template("ftl_api_autogen")?;
    let api_lib_rs = template.render(context! {
        apps => apps,
    })?;
    std::fs::write(&args.api_autogen_outfile, api_lib_rs)?;
    run_rustfmt(&args.api_autogen_outfile)?;

    Ok(())
}
