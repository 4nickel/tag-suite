mod dsl;
mod context;

pub mod import {
    pub use super::super::import::*;
}

pub mod export {
    pub use super::dsl::Dsl;
    pub use super::context::Context;
}
pub use export::*;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "invalid modifier: '{}'", c)]
        InvalidModifier { c: char, },
        #[fail(display = "invalid namespace: '{}'", name)]
        InvalidNamespace { name: String, },
    }
}
