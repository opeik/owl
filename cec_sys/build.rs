use std::{env, path::PathBuf};

use cec_bootstrap::{download_libcec, BuildKind};
use color_eyre::eyre::{eyre, Result};
use target_lexicon::OperatingSystem;

fn main() -> Result<()> {
    color_eyre::install()?;

    let download_path = PathBuf::from(env::var("OUT_DIR")?);
    let lib_path = download_path.join("libcec");
    let lib_path_str = lib_path.to_string_lossy();
    let build_kind = match std::env::var("PROFILE")?.as_str() {
        "debug" => BuildKind::Debug,
        "release" => BuildKind::Release,
        _ => return Err(eyre!("unexpected build profile")),
    };

    dbg!(&lib_path, target_lexicon::HOST, build_kind);
    println!("cargo:rustc-link-search=native={lib_path_str}");
    println!("cargo:rustc-link-lib=static=cec");
    println!("cargo:rustc-link-lib=static=p8-platform");

    match (target_lexicon::HOST.operating_system, build_kind) {
        (OperatingSystem::Windows, BuildKind::Debug) => {
            println!("cargo:rustc-link-lib=dylib=msvcrtd");
        }
        (OperatingSystem::Windows, BuildKind::Release) => {
            println!("cargo:rustc-link-lib=dylib=msvcrt");
        }
        (OperatingSystem::Darwin, _) => {
            println!("cargo:rustc-link-search=framework=/Library/Frameworks");
            println!("cargo:rustc-link-lib=dylib=c++");
            println!("cargo:rustc-link-lib=framework=CoreVideo");
            println!("cargo:rustc-link-lib=framework=IOKit");
        }
        (OperatingSystem::Linux, _) => {
            println!("cargo:rustc-link-lib=dylib=stdc++");
        }
        _ => return Err(eyre!("unsupported target")),
    };

    // Building libcec from source is _painful_, so we don't!
    download_libcec(&lib_path, build_kind)?;

    Ok(())
}
