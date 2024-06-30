#![feature(
    type_alias_impl_trait,
    let_chains,
    exposed_provenance,
    strict_provenance
)]

pub use prelude::*;

pub mod cec;
pub mod job;
pub mod os;
pub mod prelude {
    pub use crate::job::{Recv, Send, Spawn};
}
