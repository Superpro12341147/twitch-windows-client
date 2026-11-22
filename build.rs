use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=assets/twitch-client.ico");
    println!("cargo:rerun-if-changed=plugins");
    println!("cargo:rerun-if-changed=themes");
    let generated = format!(
        "{}{}",
        bundled_assets("plugins", "js", "BUNDLED_PLUGINS"),
        bundled_assets("themes", "css", "BUNDLED_THEMES"),
    );
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR is not set");
    fs::write(Path::new(&out_dir).join("bundled_assets.rs"), generated)
        .expect("could not write bundled asset declarations");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resources = winres::WindowsResource::new();
        resources.set_icon("assets/twitch-client.ico");
        resources
            .compile()
            .expect("failed to embed the Windows application icon");
    }
}

fn bundled_assets(directory: &str, extension: &str, constant: &str) -> String {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let asset_dir = Path::new(&manifest_dir).join(directory);
    let mut entries = fs::read_dir(&asset_dir)
        .unwrap_or_else(|error| panic!("could not read bundled {directory}: {error}"))
        .map(|entry| entry.expect("could not read bundled asset entry").path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        })
        .collect::<Vec<_>>();
    entries.sort();

    let mut generated = format!("pub const {constant}: &[(&str, &str)] = &[\n");
    for path in entries {
        println!("cargo:rerun-if-changed={}", path.display());
        let filename = path
            .file_name()
            .expect("bundled asset has no filename")
            .to_string_lossy();
        generated.push_str(&format!(
            "    ({filename:?}, include_str!({path:?})),\n",
            path = path.to_string_lossy(),
        ));
    }
    generated.push_str("];\n");
    generated
}
