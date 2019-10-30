use super::{import::*, api};
use crate::model::tag;
use internship::IStr;

/// A tag is just a String with a few invariants.
/// Since we expect to store many thousand instances
/// of the same Tag, it is worthwhile to deduplicate
/// the strings by interning them.
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct Tag (IStr);

impl Tag {

    /// Return the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a new Tag
    pub fn new(name: &str) -> Res<Self> {
        Ok(Self(IStr::new(api::sanitize(name)?)))
    }

    pub fn ident<'a>(&'a self) -> tag::Ident<'a> {
        tag::Ident { name: self.as_str() }
    }
}

use std::borrow::Borrow;
impl Borrow<str> for Tag {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
