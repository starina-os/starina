use std::env;
use std::fmt;
use std::fmt::Write;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use ftl_types::idl;
use ftl_types::idl::IdlFile;
use minijinja::context;
use minijinja::Environment;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
struct Field {
    name: String,
    ty: String,
    is_handle: bool,
}

#[derive(Debug, Serialize, Clone)]
struct Message {
    protocol_name: String,
    name: String,
    msgid: isize,
    num_handles: isize,
    fields: Vec<Field>,
}

#[derive(Debug, Serialize, Clone)]
struct Protocol {
    name: String,
    messages: Vec<Message>,
}

fn resolve_type(ty: &idl::Ty) -> String {
    match ty {
        idl::Ty::UInt16 => "u16".to_string(),
        idl::Ty::Int32 => "i32".to_string(),
        idl::Ty::Channel => "ftl_types::idl::HandleField".to_string(),
        idl::Ty::Bytes { capacity } => format!("ftl_types::idl::BytesField<{capacity}>"),
        idl::Ty::String { capacity } => format!("ftl_types::idl::StringField<{capacity}>"),
    }
}

fn num_handles(fields: &[Field]) -> isize {
    fields
        .iter()
        .filter(|f| f.ty == "ftl_types::idl::HandleField")
        .count()
        .try_into()
        .unwrap()
}

struct CamelCase<'a>(&'a str);

impl fmt::Display for CamelCase<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut next_upper = true;
        for c in self.0.chars() {
            if c == '_' {
                next_upper = true;
            } else if next_upper {
                f.write_char(c.to_ascii_uppercase())?;
                next_upper = false;
            } else {
                f.write_char(c)?;
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
            ty: resolve_type(&f.ty),
            is_handle: matches!(f.ty, idl::Ty::Channel),
        });
    }

    fields
}

fn find_idl_file() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut dir = manifest_dir.as_path();
    while let Some(parent_dir) = dir.parent() {
        let idl_path = parent_dir.join("idl.json");
        if idl_path.exists() {
            return Ok(idl_path);
        }

        dir = parent_dir;
    }

    anyhow::bail!("idl.json not found in any parent directory of CARGO_MANIFEST_DIR");
}

fn do_generate(for_kernel: bool) -> Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("autogen.rs");
    let idl_path = find_idl_file()?;

    let idl_file = File::open(idl_path)?;
    let idl: IdlFile = serde_json::from_reader(&idl_file)?;

    let mut next_msgid = 1;
    let mut messages = Vec::new();
    let mut protocols = Vec::new();
    for protocol in idl.protocols {
        let mut protocol_messages = Vec::new();
        if let Some(oneways) = &protocol.oneways {
            for oneway in oneways {
                let fields = visit_fields(&oneway.fields);
                protocol_messages.push(Message {
                    protocol_name: protocol.name.clone(),
                    name: format!("{}", CamelCase(&oneway.name)),
                    msgid: next_msgid,
                    num_handles: num_handles(&fields),
                    fields,
                });
                next_msgid += 1;
            }
        }

        if let Some(rpcs) = &protocol.rpcs {
            for rpc in rpcs {
                let req_fields = visit_fields(&rpc.request.fields);
                protocol_messages.push(Message {
                    protocol_name: protocol.name.clone(),
                    name: format!("{}", CamelCase(&rpc.name)),
                    msgid: next_msgid,
                    num_handles: num_handles(&req_fields),
                    fields: req_fields,
                });
                next_msgid += 1;

                let reply_fields = visit_fields(&rpc.response.fields);
                protocol_messages.push(Message {
                    protocol_name: protocol.name.clone(),
                    name: format!("{}Reply", CamelCase(&rpc.name)),
                    msgid: next_msgid,
                    num_handles: num_handles(&reply_fields),
                    fields: reply_fields,
                });
                next_msgid += 1;
            }
        }

        messages.extend(protocol_messages.iter().cloned());
        protocols.push(Protocol {
            name: protocol.name,
            messages: protocol_messages.clone(),
        });
    }

    let mut j2env = Environment::new();
    j2env
        .add_template("autogen", include_str!("autogen.rs.j2"))
        .unwrap();
    let template = j2env.get_template("autogen")?;
    let lib_rs = template
        .render(context! {
            messages => messages,
            protocols => protocols,
            generate_for_kernel => for_kernel,
        })
        .context("failed to generate autogen")?;

    std::fs::write(&dest_path, lib_rs)?;

    Ok(())
}

pub fn generate_for_kernel() -> Result<()> {
    do_generate(true)
}

pub fn generate_for_app() -> Result<()> {
    do_generate(false)
}
