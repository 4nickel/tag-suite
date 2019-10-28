mod dsl;
pub use dsl::Dsl;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "invalid modifier: '{}'", c)]
        InvalidModifier { c: char, },
        #[fail(display = "invalid namespace: '{}'", name)]
        InvalidNamespace { name: String, },
    }
}

pub mod import {
    pub use super::super::import::*;
}
