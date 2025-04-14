use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

use wizer::Wizer;

const QUICKJS_URL: &str =
    "https://github.com/quickjs-ng/quickjs/releases/download/v0.9.0/quickjs-amalgam.zip";
const SYSROOT_URL: &str = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sysroot-24.0.tar.gz";
const COMPILER_RT_URL: &str = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/libclang_rt.builtins-wasm32-wasi-24.0.tar.gz";
const COMPILRT_RT_FILENAME: &str = "libclang_rt.builtins-wasm32.a";

pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=app.js");
    println!("cargo:rerun-if-changed=main.c");

    let package_dir =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let out_dir = package_dir.join("deps");

    let app_wasm_path = package_dir.join("app.wasm");
    std::fs::remove_file(&app_wasm_path).ok();

    let quickjs_dir = out_dir.join("quickjs");
    if !quickjs_dir.exists() {
        download_and_extract(QUICKJS_URL, &quickjs_dir, None);
    }

    let rt_dir = out_dir.join("compiler_rt");
    if !rt_dir.join(COMPILRT_RT_FILENAME).exists() {
        download_and_extract(COMPILER_RT_URL, &rt_dir, Some(1));
    }

    let sysroot_dir = out_dir.join("sysroot");
    if !sysroot_dir.exists() {
        download_and_extract(SYSROOT_URL, &sysroot_dir, Some(1));
    }

    eprintln!("Compiling with clang");
    let unoptimized_wasm_path = out_dir.join("unoptimized.wasm");
    let clang_status = Command::new("/opt/homebrew/opt/llvm/bin/clang")
        .arg(package_dir.join("main.c"))
        .arg(quickjs_dir.join("quickjs-amalgam.c"))
        .arg("-I")
        .arg(&quickjs_dir)
        .arg("-std=c23")
        .arg("-O2")
        .arg("-flto=thin")
        .arg("-fomit-frame-pointer")
        .arg("-fmerge-all-constants")
        .arg("-mbulk-memory")
        .arg("--target=wasm32-wasi")
        .arg(&format!("--sysroot={}", sysroot_dir.display()))
        .arg("-nodefaultlibs")
        .arg("-L")
        .arg(&rt_dir)
        .arg("-D_GNU_SOURCE")
        .arg("-DQJS_BUILD_LIBC")
        .arg("-D_WASI_EMULATED_SIGNAL")
        .arg("-lwasi-emulated-signal")
        .arg("-lc")
        .arg("-lclang_rt.builtins-wasm32")
        .arg("-o")
        .arg(&unoptimized_wasm_path)
        .status()
        .expect("failed to execute clang");

    if !clang_status.success() {
        panic!("clang exited with status: {}", clang_status);
    }

    let unoptimized_wasm =
        fs::read(out_dir.join("unoptimized.wasm")).expect("failed to load unoptimized WASM file");

    eprintln!("Running wizer");
    eprintln!("unoptimized_wasm: {}", unoptimized_wasm_path.display());
    let wizer_status = Command::new("wizer")
        .arg("--allow-wasi")
        .arg(unoptimized_wasm_path)
        .arg("-o")
        .arg(&app_wasm_path)
        .status()
        .expect("failed to execute wizer");

    if !wizer_status.success() {
        panic!("wizer exited with status: {}", wizer_status);
    }
    eprintln!("Wizer finished successfully");

    // let wizered_wasm = Wizer::new()
    //     .wasm_bulk_memory(true)
    //     .allow_wasi(true)
    //     .unwrap()
    //     .run(&unoptimized_wasm)
    //     .expect("wizer failed");
    //
    // fs::write(&app_wasm_path, wizered_wasm).unwrap();
}

static TEMP_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR not set");
    let temp_dir = PathBuf::from(out_dir).join("build-rs-temp");
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
    temp_dir
});

fn download_and_extract(url: &str, dest_dir: &Path, strip_components: Option<u32>) {
    println!("downloading from {}", url);

    let download_file_path = TEMP_DIR.join("downloaded_file");
    let curl_status = Command::new("curl")
        .arg("-sfL")
        .arg("-o")
        .arg(&download_file_path)
        .arg(url)
        .status()
        .expect("failed to download file with curl");

    if !curl_status.success() {
        panic!("curl exited with non-zero status: {}", curl_status);
    }

    fs::create_dir_all(dest_dir).expect("Failed to create destination directory");

    enum FileType {
        Tar(&'static str),
        Zip,
    }

    let file_type = if url.ends_with(".tar.gz") {
        FileType::Tar("z")
    } else if url.ends_with(".tar.bz2") {
        FileType::Tar("j")
    } else if url.ends_with(".tar.xz") {
        FileType::Tar("J")
    } else if url.ends_with(".zip") {
        FileType::Zip
    } else if url.ends_with(".tar") {
        FileType::Tar("")
    } else {
        panic!("unexpected file extension: {}", url);
    };

    let extract_status = match file_type {
        FileType::Tar(flags) => {
            Command::new("tar")
                .arg(flags)
                .arg(&download_file_path)
                .arg("--strip-components")
                .arg(strip_components.unwrap_or(0).to_string())
                .arg("-C")
                .arg(dest_dir)
                .status()
                .expect("failed to extract file with tar")
        }
        FileType::Zip => {
            debug_assert!(
                strip_components.is_none(),
                "strip_components is not supported for zip files"
            );

            Command::new("unzip")
                .arg("-o")
                .arg(&download_file_path)
                .arg("-d")
                .arg(dest_dir)
                .status()
                .expect("failed to extract file with unzip")
        }
    };

    if !extract_status.success() {
        panic!("tar exited with non-zero status: {}", extract_status);
    }

    eprintln!("successfully extracted files to {}", dest_dir.display());
}
