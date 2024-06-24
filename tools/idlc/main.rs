use std::fmt;
use std::fs::File;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use ftl_types::idl::IdlFile;
use ftl_types::idl::{self};
use minijinja::context;
use minijinja::Environment;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Field {
    name: String,
    ty: String,
}

#[derive(Debug, Serialize)]
struct Message {
    name: String,
    msgid: isize,
    fields: Vec<Field>,
}

fn resolve_type_name(ty: &idl::Ty) -> String {
    match ty {
        idl::Ty::Int32 => "i32".to_string(),
        _ => panic!("Unknown type: {:?}", ty),
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(index = 1)]
    idl_file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let idl_file = File::open(args.idl_file)?;
    let protocol: IdlFile = serde_json::from_reader(&idl_file)?;

    let mut j2env = Environment::new();
    j2env
        .add_template("template", include_str!("templates/ftl_autogen.rs.j2"))
        .unwrap();

    let mut messages = Vec::new();
    for (i, protocol) in protocol.protocols.iter().enumerate() {
        for rpc in &protocol.rpcs {
            let request_name = format!("{}Request", CamelCase(&rpc.name));
            let reply_name = format!("{}Reply", CamelCase(&rpc.name));

            messages.push(Message {
                name: request_name,
                msgid: i as isize, // TODO: derive a globally unique ID
                fields: rpc
                    .request
                    .fields
                    .iter()
                    .map(|f| {
                        Field {
                            name: f.name.clone(),
                            ty: resolve_type_name(&f.ty),
                        }
                    })
                    .collect(),
            });

            messages.push(Message {
                name: reply_name,
                msgid: i as isize, // TODO: derive a globally unique ID
                fields: rpc
                    .response
                    .fields
                    .iter()
                    .map(|f| {
                        Field {
                            name: f.name.clone(),
                            ty: resolve_type_name(&f.ty),
                        }
                    })
                    .collect(),
            });
        }
    }

    let template = j2env.get_template("template")?;
    let lib_rs = template.render(context! {
        messages => messages,
    })?;

    println!("{}", lib_rs);

    Ok(())
}
