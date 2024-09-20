#![feature(let_chains)]

use std::{
    env,
    io::Cursor,
    path::{Path, PathBuf},
};

use cfg_if::cfg_if;
use color_eyre::eyre::{eyre, Result};
use target_lexicon::OperatingSystem;

#[derive(Debug)]
pub enum BuildKind {
    Debug,
    Release,
}

cfg_if! {
    if #[cfg(debug_assertions)] {
        const BUILD_KIND: BuildKind = BuildKind::Debug;
    } else {
        const BUILD_KIND: BuildKind = BuildKind::Release;
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let download_path = PathBuf::from(env::var("OUT_DIR")?);
    let lib_path = download_path.join("libcec");
    let lib_path_str = lib_path.to_string_lossy();
    dbg!(&lib_path);

    println!("cargo:rustc-link-search=native={lib_path_str}");
    println!("cargo:rustc-link-lib=static=cec");
    println!("cargo:rustc-link-lib=static=p8-platform");
    dbg!(target_lexicon::HOST);

    match (target_lexicon::HOST.operating_system, BUILD_KIND) {
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
            println!("cargo:rustc-link-lib=dylib=c++");
        }
        _ => return Err(eyre!("unsupported target")),
    };

    // Building libcec from source is _painful_.
    download_libcec(&lib_path)?;

    Ok(())
}

fn download_libcec<P: AsRef<Path>>(path: P) -> Result<()> {
    let target = target_lexicon::HOST.to_string();
    let build_kind = BUILD_KIND;

    let url = format!("https://github.com/opeik/libcec-vendor/releases/download/v0.1.0/libcec-6.0.2-{target}-{build_kind}.zip");
    dbg!(target, build_kind, &url);
    if !path.as_ref().exists() {
        let file = reqwest::blocking::get(url)?.bytes()?;
        zip_extract::extract(Cursor::new(file), path.as_ref(), true)?;
    }

    Ok(())
}

impl std::fmt::Display for BuildKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BuildKind::Debug => "debug",
            BuildKind::Release => "release",
        };

        write!(f, "{s}")
    }
}
