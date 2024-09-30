use std::{env, path::PathBuf};

use cec_bootstrap::{fetch_libcec, BuildKind};
use color_eyre::eyre::{eyre, Context, Result};
use target_lexicon::OperatingSystem;

fn main() -> Result<()> {
    color_eyre::install()?;

    let download_path =
        PathBuf::from(env::var("OUT_DIR").context("env var `OUT_DIR` is undefined")?);
    let lib_path = download_path.join("libcec");
    let lib_path_str = lib_path.to_string_lossy();
    let build_kind = if cfg!(debug_assertions) {
        BuildKind::Debug
    } else {
        BuildKind::Release
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
    fetch_libcec(&lib_path, build_kind).context("failed to download libcec")?;

    Ok(())
}
