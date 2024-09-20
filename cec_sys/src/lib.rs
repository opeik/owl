mod bindings {
    #![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
    cfg_if::cfg_if! {
        if #[cfg(target_os = "windows")] {
            include!("bindings_windows.rs");
        } else if #[cfg(target_os = "macos")] {
            include!("bindings_macos.rs");
        } else if #[cfg(target_os = "linux")] {
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
