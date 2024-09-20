mod bindings {
    #![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
    cfg_if::cfg_if! {
        if #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc"))] {
            include!("x86_64-pc-windows-msvc.rs");
        } else if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
            include!("aarch64-apple-darwin.rs");
        } else if #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))] {
            include!("bindings_linux.rs");
        } else {
            compile_error!("unsupported platform");
        }
    }
}

pub use crate::bindings::*;

#[cfg(test)]
mod tests {
    use crate::CEC_LIB_VERSION_MAJOR;

    #[test]
    fn check_version() {
        assert_eq!(CEC_LIB_VERSION_MAJOR, 6);
    }
}
