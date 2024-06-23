mod bindings {
    #![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
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
