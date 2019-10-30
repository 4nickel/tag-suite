use super::import::*;
use crate::model::{file, tag, FileTag};

/// Stores Insertable values during update
pub struct Ins<'u> {
    pub files: Vec<file::Insert<'u>>,
    pub tags: Vec<tag::Insert<'u>>,
    pub filetags: Vec<FileTag>,
}

impl<'u> Ins<'u> {
    /// Create a new instance
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            tags: Vec::new(),
            filetags: Vec::new(),
        }
    }
}

/// Stores the keys to delete during update
pub struct Del<'u> {
    pub files: Vec<&'u str>,
    pub tags: Vec<&'u str>,
    pub filetags: Vec<Ids>,
}

impl<'u> Del<'u> {
    /// Create a new instance
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            tags: Vec::new(),
            filetags: Vec::new(),
        }
    }
}
