#![feature(type_alias_impl_trait)]
#![feature(let_chains)]

pub use prelude::*;

pub mod cec;
pub mod job;
pub mod os;
pub mod prelude {
    pub use crate::job::{Recv, Send, Spawn};
}
