// TODO: auto geenrate this file from app.toml
use starina::spec::AppImage;
use starina::spec::AppSpec;

pub const APP_SPEC: AppSpec = AppSpec {
    name: "hello_wasm",
    image: AppImage::Wasm {
        wasm: include_bytes!("app.wasm"),
    },
    env: &[],
    exports: &[],
};
