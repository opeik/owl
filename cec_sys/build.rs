#![feature(let_chains)]

use std::{
    env,
    io::Cursor,
    path::{Path, PathBuf},
};

use cfg_if::cfg_if;
use color_eyre::eyre::{eyre, Result};

fn main() -> Result<()> {
    color_eyre::install()?;
    let download_path = PathBuf::from(env::var("OUT_DIR")?);
    let lib_path = download_path.join("libcec");
    dbg!(&lib_path);

    println!(
        "cargo:rustc-link-search=native={}",
        lib_path.to_string_lossy()
    );

    println!("cargo:rustc-link-lib=static=cec");
    println!("cargo:rustc-link-lib=static=p8-platform");

    match std::env::consts::OS {
        "windows" => {}
        "macos" => {
            println!("cargo:rustc-link-search=framework=/Library/Frameworks");

            println!("cargo:rustc-link-lib=c++");
            println!("cargo:rustc-link-lib=framework=CoreVideo");
            println!("cargo:rustc-link-lib=framework=IOKit");
        }
        "linux" => {
            println!("cargo:rustc-link-lib=c++");
        }
        _ => return Err(eyre!("invalid arch")),
    };

    // Building libcec from source is _painful_.
    download_libcec(&lib_path)?;

    Ok(())
}

fn download_libcec<P: AsRef<Path>>(path: P) -> Result<()> {
    cfg_if! {
        if #[cfg(debug_assertions)] {
            let kind = "debug";
        } else {
            let kind = "release";
        }
    }

    let os = match std::env::consts::OS {
        x @ ("windows" | "macos" | "linux") => x,
        _ => return Err(eyre!("invalid arch")),
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => return Err(eyre!("invalid arch")),
    };

    let url = format!("https://github.com/opeik/libcec-vendor/releases/download/v0.1.0/libcec-6.0.2-{os}-{arch}-{kind}.zip");
    dbg!(&kind, &arch, &url);
    if !path.as_ref().exists() {
        let file = reqwest::blocking::get(url)?.bytes()?;
        zip_extract::extract(Cursor::new(file), path.as_ref(), true)?;
    }

    Ok(())
}
