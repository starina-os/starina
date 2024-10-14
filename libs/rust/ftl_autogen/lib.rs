use std::env;
use std::fmt;
use std::fmt::Write;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use ftl_types::spec::InterfaceSpec;
use ftl_types::spec::MessageField;
use ftl_types::spec::MessageFieldType;
use ftl_types::spec::MessageType;
use ftl_types::spec::Spec;
use ftl_types::spec::SpecFile;
use minijinja::context;
use minijinja::Environment;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
struct JinjaField {
    name: String,
    ty: String,
    is_handle: bool,
}

#[derive(Debug, Serialize, Clone)]
struct JinjaMessage {
    interface_name: String,
    name: String,
    msgid: isize,
    num_handles: isize,
    reply_message_name: Option<String>,
    fields: Vec<JinjaField>,
}

#[derive(Debug, Serialize, Clone)]
struct JinjaInterface {
    name: String,
    messages: Vec<JinjaMessage>,
}

fn resolve_type(ty: &MessageFieldType) -> String {
    match ty {
        MessageFieldType::UInt8 => "u8".to_string(),
        MessageFieldType::UInt16 => "u16".to_string(),
        MessageFieldType::UInt32 => "u32".to_string(),
        MessageFieldType::Int8 => "i8".to_string(),
        MessageFieldType::Int16 => "i16".to_string(),
        MessageFieldType::Int32 => "i32".to_string(),
        MessageFieldType::Channel => "ftl_types::idl::HandleField".to_string(),
        MessageFieldType::Bytes { capacity } => format!("ftl_types::idl::BytesField<{capacity}>"),
        MessageFieldType::String { capacity } => format!("ftl_types::idl::StringField<{capacity}>"),
    }
}

fn is_handle(ty: &MessageFieldType) -> bool {
    matches!(ty, MessageFieldType::Channel)
}

fn num_handles(fields: &[MessageField]) -> isize {
    fields
        .iter()
        .filter(|f| is_handle(&f.ty))
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

fn visit_fields(idl_fields: &[MessageField]) -> Vec<JinjaField> {
    let mut fields = Vec::with_capacity(idl_fields.len());
    for f in idl_fields {
        fields.push(JinjaField {
            name: f.name.clone(),
            ty: resolve_type(&f.ty),
            is_handle: is_handle(&f.ty),
        });
    }

    fields
}

fn find_spec_dir() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut dir = manifest_dir.as_path();
    while let Some(parent_dir) = dir.parent() {
        let path = parent_dir.join("spec/interfaces");
        if path.is_dir() {
            return Ok(path);
        }

        dir = parent_dir;
    }

    anyhow::bail!(
        "spec/interfaces directory not found in any parent directory of CARGO_MANIFEST_DIR"
    );
}

fn visit_interface(
    interface_name: &str,
    spec: &InterfaceSpec,
    next_msgid: &mut isize,
) -> Result<JinjaInterface> {
    let mut jinja_messages = Vec::with_capacity(spec.messages.len());
    for message in &spec.messages {
        let reply_message_name = format!("{}Reply", CamelCase(&message.name));

        jinja_messages.push(JinjaMessage {
            interface_name: interface_name.to_string(),
            name: format!("{}", CamelCase(&message.name)),
            msgid: *next_msgid,
            num_handles: num_handles(&message.params),
            fields: visit_fields(&message.params),
            reply_message_name: if message.ty == MessageType::Call {
                Some(reply_message_name.clone())
            } else {
                None
            },
        });
        *next_msgid += 1;

        match (message.returns.as_ref(), &message.ty) {
            (Some(returns), MessageType::Call) => {
                jinja_messages.push(JinjaMessage {
                    interface_name: interface_name.to_string(),
                    name: reply_message_name,
                    msgid: *next_msgid,
                    num_handles: num_handles(returns),
                    fields: visit_fields(returns),
                    reply_message_name: None,
                });
                *next_msgid += 1;
            }
            (Some(_), _) => {
                panic!(
                    "{}:{}: non-call message must not have \"returns\"",
                    interface_name, message.name
                );
            }
            (None, MessageType::Call) => {
                panic!(
                    "{}:{}: call message must have \"returns\"",
                    interface_name, message.name
                );
            }
            (None, _) => {
                // Non-call message without returns. Nothing to do.
            }
        }
    }

    Ok(JinjaInterface {
        name: interface_name.to_string(),
        messages: jinja_messages,
    })
}

fn do_generate(for_kernel: bool) -> Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("autogen.rs");
    let spec_dir = find_spec_dir()?;

    let mut next_msgid = 1;
    let mut interfaces = Vec::new();
    for dentry in spec_dir
        .read_dir()
        .context("failed to readdir spec/interfaces")?
    {
        let dentry = dentry.unwrap();
        let spec_file = File::open(dentry.path())
            .with_context(|| format!("failed to open spec file {}", dentry.path().display()))?;
        let spec: SpecFile = serde_yaml::from_reader(spec_file)
            .with_context(|| format!("failed to parse spec file {}", dentry.path().display()))?;

        let interface_spec = match spec.spec {
            Spec::Interface(spec) => spec,
            _ => {
                panic!("{}: expected interface spec", dentry.path().display());
            }
        };

        let iface = visit_interface(&spec.name, &interface_spec, &mut next_msgid)
            .with_context(|| format!("failed to process interface {}", dentry.path().display()))?;

        interfaces.push(iface);
    }

    let mut all_messages = Vec::new();
    for iface in &interfaces {
        all_messages.extend(iface.messages.iter().cloned());
    }

    let mut jinja_env = Environment::new();
    jinja_env
        .add_template("autogen", include_str!("autogen.rs.j2"))
        .unwrap();
    let template = jinja_env.get_template("autogen")?;
    let lib_rs = template
        .render(context! {
            messages => all_messages,
            interfaces => interfaces,
            generate_for_kernel => for_kernel,
        })
        .context("failed to generate autogen")?;

    std::fs::write(&dest_path, lib_rs)?;

    Ok(())
}

fn print_cargo_instructions() {
    println!("cargo:rerun-if-changed=spec/interfaces");
}

pub fn generate_for_kernel() -> Result<()> {
    print_cargo_instructions();
    do_generate(true)
}

pub fn generate_for_app() -> Result<()> {
    print_cargo_instructions();
    do_generate(false)
}
