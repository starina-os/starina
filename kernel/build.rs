use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use ftl_types::spec::AppSpec;
use ftl_types::spec::Spec;
use ftl_types::spec::SpecFile;
use minijinja::context;
use minijinja::Environment;

const AUTOGEN_TEMPLATE: &str = r#"

    macro_rules! aligned_include_bytes {
        ($file:expr, $align:expr) => { {
            // A wrapper to ensure the inner type is aligned to $align bytes.
            #[repr(C, align($align))]
            struct Aligned<T: ?Sized>(T);

            // A include_bytes!() wrapped in the Aligned type. Compiler will
            // ensure the bytes ([u8]) are well aligned.
            const ALIGNED: &Aligned<[u8]> = &Aligned(*include_bytes!($file));

            // Now we have a &[u8] that is aligned to $align bytes.
            &ALIGNED.0
        } };
    }

    const STARTUP_APPS: &[crate::startup::AppTemplate] = {
        #[allow(unused)]
        use crate::startup::{AppTemplate, AppName, ServiceName, WantedHandle, WantedDevice};

        &[
        {% for name, app in startup_apps %}
            AppTemplate {
                name: AppName("{{ name }}"),
                elf_file: aligned_include_bytes!("{{ build_dir }}/apps/{{ name }}.elf", 4096),
                provides: &[
                    {% for service_name in app.provides %}
                        ServiceName("{{ service_name }}"),
                    {% endfor %}
                ],
                handles: &[
                    {% for depend in app.depends %}
                        {% if depend.type == "service" %}
                            WantedHandle::Service(ServiceName("{{ depend.protocol }}")),
                        {% endif %}
                    {% endfor %}
                ],
                devices: &[
                    {% for depend in app.depends %}
                        {% if depend.type == "device" and depend.device_tree %}
                            {% for compatible in depend.device_tree.compatible %}
                                WantedDevice::DeviceTreeCompatible("{{ compatible }}"),
                            {% endfor %}
                        {% endif %}
                    {% endfor %}
                ],
            },
        {% endfor %}
        ]
    };
"#;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("autogen.rs");

    let build_dir = PathBuf::from(env::var("BUILD_DIR").expect("$BUILD_DIR is not set"));
    assert!(build_dir.is_absolute());

    let startup_apps_str = env::var("STARTUP_APP_DIRS").expect("$STARTUP_APPS is not set");
    let mut startup_apps: Vec<(String, AppSpec)> = Vec::new();
    for app_dir in startup_apps_str.split_ascii_whitespace() {
        let app_dir = Path::new(app_dir);
        if !app_dir.is_absolute() {
            panic!(
                "BUG: non-absolute path found({}) in STARTUP_APP_DIRS: {}",
                app_dir.display(),
                startup_apps_str
            );
        }

        let app_spec_path = app_dir.join("app.spec.json");
        let app_spec_str = fs::read_to_string(&app_spec_path)
            .with_context(|| format!("failed to read app spec: {}", app_spec_path.display()))
            .unwrap();
        let spec: SpecFile = serde_json::from_str(&app_spec_str)
            .with_context(|| format!("failed to parse app spec: {}", app_spec_path.display()))
            .unwrap();
        let app_spec: AppSpec = match spec.spec {
            Spec::App(app_spec) => app_spec,
            // spec => {
            //     panic!("{}: expected app spec, found {:?}", app_spec_path.display(), spec);
            // }
        };

        startup_apps.push((spec.name, app_spec));
    }

    let mut j2env = Environment::new();
    j2env.add_template("autogen", AUTOGEN_TEMPLATE).unwrap();
    let autogen = j2env
        .get_template("autogen")
        .unwrap()
        .render(context! {
            build_dir => build_dir,
            startup_apps => startup_apps,
        })
        .unwrap();

    println!("cargo::rerun-if-env-changed=STARTUP_APP_DIRS");
    fs::write(dest_path, &autogen).unwrap();
}
