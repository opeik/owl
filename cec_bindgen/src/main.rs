#![feature(let_chains)]

use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use bcmp::AlgoSpec;
use bindgen::callbacks::ParseCallbacks;
use cfg_if::cfg_if;
use clap::Parser;
use color_eyre::eyre::Result;

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

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "cec_bindgen")]
    src_path: String,
    #[arg(short, long)]
    dest_path: Option<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::try_parse()?;

    let tmp_dir = tempfile::tempdir()?;
    let build_path = PathBuf::from(tmp_dir.path());
    let src_path = PathBuf::from(args.src_path);
    let lib_path = build_path.join("libcec");
    let out_path = PathBuf::from(match args.dest_path {
        Some(x) => x,
        None => format!("cec_sys/src/bindings/{}.rs", target_lexicon::HOST),
    });

    dbg!(build_path);

    // Building libcec from source is _painful_, so we don't!
    download_libcec(&lib_path)?;
    run_bindgen(&src_path, &lib_path, &out_path)?;
    dbg!(&out_path);

    Ok(())
}

fn download_libcec<P: AsRef<Path>>(path: P) -> Result<()> {
    let target = target_lexicon::HOST.to_string();
    let build_kind = BUILD_KIND;

    let url = format!("https://github.com/opeik/owl/releases/download/libcec-v6.0.2/libcec-6.0.2-{target}-{build_kind}.zip");
    dbg!(target, build_kind, &url);
    if !path.as_ref().exists() {
        let file = reqwest::blocking::get(url)?.bytes()?;
        zip_extract::extract(Cursor::new(file), path.as_ref(), true)?;
    }

    Ok(())
}

fn run_bindgen<P: AsRef<Path>>(src_path: P, lib_path: P, out_path: P) -> Result<()> {
    const ALLOW_REGEX: &str = "(libcec|cec|CEC|LIBCEC)_.*";
    let include_path = lib_path.as_ref().join("include");
    let header_path = src_path.as_ref().join("wrapper.h");

    let bindings = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .allowlist_type(ALLOW_REGEX)
        .allowlist_function(ALLOW_REGEX)
        .allowlist_var(ALLOW_REGEX)
        .rustified_enum(".*")
        .prepend_enum_name(false)
        .sort_semantically(true)
        .merge_extern_blocks(true)
        .derive_default(true)
        .derive_debug(true)
        .derive_copy(true)
        .clang_args([
            "--verbose",
            "--include-directory",
            &include_path.to_string_lossy(),
        ])
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(TidySymbols))
        .generate()?;

    bindings.write_to_file(out_path.as_ref())?;

    Ok(())
}

#[derive(Debug)]
struct TidySymbols;

impl ParseCallbacks for TidySymbols {
    fn will_parse_macro(&self, _name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        bindgen::callbacks::MacroParsingBehavior::Default
    }

    fn generated_name_override(
        &self,
        _item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        None
    }

    fn generated_link_name_override(
        &self,
        _item_info: bindgen::callbacks::ItemInfo<'_>,
    ) -> Option<String> {
        None
    }

    fn int_macro(&self, _name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
        None
    }

    fn enum_variant_behavior(
        &self,
        _enum_name: Option<&str>,
        _original_variant_name: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<bindgen::callbacks::EnumVariantCustomBehavior> {
        None
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        variant_name: &str,
        _value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        let exceptional_prefixes = [
            "CEC_AUDIO_RATE_",
            "CEC_AUDIO_",
            "ADAPTERTYPE_",
            "CEC_VENDOR_",
            "CEC_DEVICE_STATUS_",
            "CECDEVICE_",
        ];
        let exception = exceptional_prefixes
            .iter()
            .flat_map(|prefix| {
                variant_name
                    .strip_prefix(prefix)
                    .map(|variant| (prefix, variant))
            })
            .max_by(|(a, _), (b, _)| a.len().cmp(&b.len()));

        if let Some((_prefix, variant)) = exception {
            return Some(variant.to_owned());
        }

        let prefixes = ["enum ", "LIB"];
        let mut enum_name = enum_name.unwrap();
        for prefix in prefixes {
            if let Some(x) = enum_name.strip_prefix(prefix) {
                enum_name = x;
            }
        }
        let enum_name = enum_name.to_uppercase();

        let variant_name = variant_name.trim();
        let substring = bcmp::longest_common_substring(
            variant_name.as_bytes(),
            enum_name.as_bytes(),
            AlgoSpec::HashMatch(2),
        );

        let prefix = format!(
            "{}_",
            &variant_name[substring.first_pos..substring.first_end()]
        );

        if let Some(x) = variant_name.strip_prefix(&prefix) {
            if x.chars().next().unwrap().is_numeric() {
                Some(format!("_{x}"))
            } else {
                Some(x.to_string())
            }
        } else {
            None
        }
    }

    fn item_name(&self, _name: &str) -> Option<String> {
        None
    }

    fn blocklisted_type_implements_trait(
        &self,
        _name: &str,
        _derive_trait: bindgen::callbacks::DeriveTrait,
    ) -> Option<bindgen::callbacks::ImplementsTrait> {
        None
    }

    fn add_derives(&self, _info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        vec![]
    }

    fn process_comment(&self, _comment: &str) -> Option<String> {
        None
    }

    fn str_macro(&self, _name: &str, _value: &[u8]) {}
    fn func_macro(&self, _name: &str, _value: &[&[u8]]) {}
    fn include_file(&self, _filename: &str) {}
    fn read_env_var(&self, _key: &str) {}
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
