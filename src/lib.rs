#![feature(type_alias_impl_trait)]

pub mod cec;
pub mod os;

pub mod prelude {
    pub use crate::os::Spawn;
}

pub use prelude::*;
