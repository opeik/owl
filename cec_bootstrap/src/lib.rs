use std::{io::Cursor, path::Path};

use cfg_if::cfg_if;
use color_eyre::eyre::Result;

#[derive(Debug)]
pub enum BuildKind {
    Debug,
    Release,
}

cfg_if! {
    if #[cfg(debug_assertions)] {
        pub const BUILD_KIND: BuildKind = BuildKind::Debug;
    } else {
        pub const BUILD_KIND: BuildKind = BuildKind::Release;
    }
}

pub fn download_libcec<P: AsRef<Path>>(path: P) -> Result<()> {
    let target = target_lexicon::HOST.to_string();
    let build_kind = BUILD_KIND;

    let url = format!("https://github.com/opeik/owl/releases/download/libcec-v6.0.2/libcec-v6.0.2-{target}-{build_kind}.zip");
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
            Self::Debug => "debug",
            Self::Release => "release",
        };

        write!(f, "{s}")
    }
}
