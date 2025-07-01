use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use serde::Serialize;
use tempfile::NamedTempFile;
use tempfile::TempDir;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    media_type: String,
    config: ConfigDescriptor,
    layers: Vec<LayerDescriptor>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigDescriptor {
    media_type: String,
    digest: String,
    size: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LayerDescriptor {
    media_type: String,
    digest: String,
    size: u64,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LINUXRUN_IMAGE");
    println!("cargo:rerun-if-env-changed=LINUXRUN_ARCH");

    if env::var_os("STARINA_RUN_SH").is_none() {
        // If STARINA_RUN_SH is not set, it's likely rust-analyzer triggered
        // this build script. Avoid running the build script in this case
        // not to drain your battery.
        println!("cargo:warning=Skipping build in rust-analyzer");
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let arch = env::var("LINUXRUN_ARCH").unwrap();
    let image_name = env::var("LINUXRUN_IMAGE").expect("LINUXRUN_IMAGE is not set");
    let image_slug = image_name.replace('/', "-");
    let squashfs_path = out_dir.join(format!("{image_slug}-{arch}.squashfs"));

    if !squashfs_path.exists() {
        let image_dir = download_image(&image_name, &arch);
        let flatten_dir = extract_image(image_dir.path());
        mksquashfs(flatten_dir.path(), &squashfs_path);
    }

    std::fs::copy(&squashfs_path, out_dir.join("container.squashfs")).unwrap();
}

fn download_image(image_name: &str, arch: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let status = Command::new("skopeo")
        .arg("copy")
        .arg("--override-arch")
        .arg(arch)
        .arg("--override-os")
        .arg("linux")
        .arg(image_name)
        .arg(format!("dir:{}", temp_dir.path().display()))
        .status()
        .expect("failed to run skopeo copy");

    if !status.success() {
        panic!("Skopeo copy failed for image: {}", image_name);
    }

    temp_dir
}

fn extract_image(image_dir: &Path) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    let manifest_path = image_dir.join("manifest.json");
    let manifest_file = fs::File::open(&manifest_path).expect("failed to open manifest.json");
    let manifest: Manifest =
        serde_json::from_reader(manifest_file).expect("failed to parse manifest.json");

    for layer in &manifest.layers {
        let blob_name = layer.digest.replace("sha256:", "");
        let blob_path = image_dir.join(&blob_name);
        assert!(blob_path.exists(), "blob file not found: {:?}", blob_path);

        // TODO: Support removed files.
        let status = Command::new("tar")
            .arg("-xf")
            .arg(&blob_path)
            .arg("-C")
            .arg(temp_dir.path())
            .arg("--exclude=dev/**")
            .arg("--exclude=proc/**")
            .arg("--exclude=sys/**")
            .arg("--exclude=tmp/**")
            .status()
            .expect("failed to run tar");

        if !status.success() {
            panic!("tar extraction failed for layer: {:?}", blob_path);
        }
    }

    temp_dir
}

fn mksquashfs(extracted_dir: &Path, squashfs_path: &Path) {
    let temp_squashfs = NamedTempFile::new().unwrap();
    let mksquashfs_status = Command::new("mksquashfs")
        .arg(&extracted_dir)
        .arg(temp_squashfs.path())
        .arg("-comp")
        .arg("lz4")
        .arg("-no-xattrs")
        .arg("-no-progress")
        .arg("-noappend")
        .status()
        .expect("failed to mksquashfs");

    if !mksquashfs_status.success() {
        panic!("mksquashfs failed");
    }

    temp_squashfs.persist(&squashfs_path).unwrap();
}
