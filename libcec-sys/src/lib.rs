// #[allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_version() {
        assert_eq!(CEC_LIB_VERSION_MAJOR, 6);
        assert_eq!(CEC_LIB_VERSION_MINOR, 0);
    }
}
