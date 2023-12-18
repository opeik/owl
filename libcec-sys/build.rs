#![feature(let_chains)]

use std::{
    env,
    io::Cursor,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use bindgen::callbacks::{
    DeriveInfo, DeriveTrait, EnumVariantCustomBehavior, EnumVariantValue, ImplementsTrait, IntKind,
    ItemInfo, ParseCallbacks,
};
use color_eyre::eyre::Result;
use itertools::Itertools;
use regex::Regex;

#[derive(Debug)]
enum Entry {
    Brief { comment: String },
    Param { param: String, comment: String },
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let build_path = PathBuf::from(env::var("OUT_DIR")?);
    let src_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let lib_path = build_path.join("libcec");
    let out_path = build_path.join("bindings.rs");

    println!(
        "cargo:rerun-if-changed={}",
        src_path.join("wrapper.h").to_string_lossy()
    );
    println!("cargo:rustc-link-search={}", lib_path.to_string_lossy());
    println!("cargo:rustc-link-lib=static=cec");

    // Building libcec from source is _painful_.
    download_libcec(&lib_path)?;
    run_bindgen(&src_path, &lib_path, &out_path)?;

    dbg!(&out_path);

    Ok(())
}

fn download_libcec<P: AsRef<Path>>(path: P) -> Result<()> {
    #[cfg(debug_assertions)]
    let url = "https://github.com/skraus-dev/libcec-vendor/releases/download/6.0.2/libcec-6.0.2_amd64_debug.zip";
    #[cfg(not(debug_assertions))]
    let url = "https://github.com/skraus-dev/libcec-vendor/releases/download/6.0.2/libcec-6.0.2_amd64_release.zip";

    let file = reqwest::blocking::get(url)?.bytes()?;
    zip_extract::extract(Cursor::new(file), path.as_ref(), true)?;

    Ok(())
}

fn run_bindgen<P: AsRef<Path>>(src_path: P, lib_path: P, out_path: P) -> Result<()> {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let include_path = lib_path.as_ref().join("include").join("libcec");
    let header_path = src_path.as_ref().join("wrapper.h");
    let regex = "(libcec|cec|CEC|LIBCEC)_.*";

    let bindings = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .allowlist_type(regex)
        .allowlist_function(regex)
        .allowlist_var(regex)
        .rustified_enum(regex)
        .c_naming(false)
        .sort_semantically(true)
        .merge_extern_blocks(true)
        .clang_args([
            "--verbose",
            "--include-directory",
            &include_path.to_string_lossy(),
        ])
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .parse_callbacks(Box::new(TidyBindings {}))
        .generate()?;

    bindings.write_to_file(out_path.as_ref())?;

    Ok(())
}

#[derive(Debug)]
struct TidyBindings {}
impl ParseCallbacks for TidyBindings {
    fn process_comment(&self, comment: &str) -> Option<String> {
        if let Some(line) = comment
            .strip_prefix("< ")
            .or_else(|| comment.strip_prefix("<!"))
        {
            return Some(fix_casing(line.trim()));
        }

        doxygen_to_rustdoc(comment)
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<String> {
        let enum_name = enum_name.unwrap();

        if let Some(variant) = strip_enum_prefix_exceptions(variant_name) {
            Some(variant)
        } else {
            strip_enum_prefix(enum_name, variant_name)
        }
    }

    fn item_name(&self, name: &str) -> Option<String> {
        println!("{name}");
        None
    }

    fn include_file(&self, _filename: &str) {}
    fn read_env_var(&self, _key: &str) {}
    fn str_macro(&self, _name: &str, _value: &[u8]) {}
    fn func_macro(&self, _name: &str, _value: &[&[u8]]) {}

    fn will_parse_macro(&self, _name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        bindgen::callbacks::MacroParsingBehavior::Default
    }

    fn generated_name_override(&self, _item: ItemInfo<'_>) -> Option<String> {
        None
    }

    fn generated_link_name_override(&self, _item: ItemInfo<'_>) -> Option<String> {
        None
    }

    fn int_macro(&self, _name: &str, _value: i64) -> Option<IntKind> {
        None
    }

    fn enum_variant_behavior(
        &self,
        _enum: Option<&str>,
        _name: &str,
        _value: EnumVariantValue,
    ) -> Option<EnumVariantCustomBehavior> {
        None
    }

    fn blocklisted_type_implements_trait(
        &self,
        _name: &str,
        _derive_trait: DeriveTrait,
    ) -> Option<ImplementsTrait> {
        None
    }

    fn add_derives(&self, _info: &DeriveInfo<'_>) -> Vec<String> {
        vec![]
    }
}

fn strip_enum_prefix_exceptions(variant_name: &str) -> Option<String> {
    // Hardcoded list of enum variant prefixes that don't match the enum name.
    let prefixes = [
        "ADAPTERTYPE_",
        "CEC_ALERT_",
        "CEC_AUDIO_RATE_",
        "CEC_AUDIO_",
        "CEC_DEVICE_STATUS_",
        "CEC_LOG_",
        "CEC_PARAMETER_",
        "CEC_VENDOR_",
        "CECDEVICE_",
        "CEC_TIMER_CLEARED_STATUS_DATA_TIMER_",
    ];

    let matches = prefixes
        .iter()
        .filter_map(|prefix| {
            let variant = variant_name.strip_prefix(prefix).map(|x| x.to_owned());
            variant.map(|x| (*prefix, x))
        })
        .max_by(|(prefix_lhs, _), (prefix_rhs, _)| prefix_lhs.len().cmp(&prefix_rhs.len()));

    matches.map(|(_prefix, variant)| variant)
}

fn strip_enum_prefix(enum_name: &str, variant_name: &str) -> Option<String> {
    let prefix = enum_name
        .strip_prefix("enum ")
        .unwrap_or(enum_name)
        .to_uppercase()
        .to_owned();

    let variant = variant_name
        .strip_prefix(&format!("{prefix}_"))
        .unwrap_or(variant_name)
        .to_string();

    if variant.chars().next().unwrap().is_numeric() {
        Some(format!("_{variant}"))
    } else {
        Some(variant)
    }
}

fn doxygen_to_rustdoc(comment: &str) -> Option<String> {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = REGEX.get_or_init(|| Regex::new(r#"@(\w+)\s+(\w.+?)\s+(\w.+)"#).unwrap());

    let captures = regex
        .captures_iter(comment)
        .map(|c| {
            let (_, [kind, param, line]) = c.extract();
            (kind, param, line)
        })
        .collect::<Vec<(&str, &str, &str)>>();

    if captures.is_empty() {
        return None;
    }

    let formatted_comment = captures
        .iter()
        .flat_map(|(kind, param, line)| match *kind {
            "param" => Some(Entry::Param {
                param: (*param).to_owned(),
                comment: fix_casing((*line).trim()),
            }),
            "brief" => Some(Entry::Brief {
                comment: fix_casing(format!("{param} {line}").trim()),
            }),
            _ => None,
        })
        .map(|entry| match entry {
            Entry::Brief { comment } => format!("{comment}\n\n# Parameters\n"),
            Entry::Param { param, comment } => format!("- `{param}`: {comment}"),
        })
        .join("\n");

    Some(formatted_comment)
}

fn fix_casing(s: &str) -> String {
    // Gross but it'll do.
    let mut v: Vec<char> = s.chars().collect();
    v[0] = v[0].to_uppercase().next().unwrap();

    if let Some(x) = v.last()
        && *x != '.'
    {
        v.push('.');
    }

    v.into_iter().collect::<String>()
}
