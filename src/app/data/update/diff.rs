use super::import::*;
use crate::{app::attr::{File as Attributes}, util::collections};

pub type DiffedFileTags<'d> = collections::Diffed<'d, (&'d str, &'d str)>;
pub type DiffedStrIds<'d> = collections::Diffed<'d, &'d str>;

/// Sets of found tags/attrs in the
/// attrsystem and database. Used to
/// generate 'diffs', which we then
/// use to create the insert and delete
/// statements.
pub struct Diff<'u> {
    files: collections::Diff<&'u str>,
    tags: collections::Diff<&'u str>,
    filetags: collections::Diff<(&'u str, &'u str)>,
}

impl<'u> Diff<'u> {

    pub fn new() -> Self {
        Self {
            files: collections::Diff::new(),
            tags: collections::Diff::new(),
            filetags: collections::Diff::new(),
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.files.shrink_to_fit();
        self.tags.shrink_to_fit();
        self.filetags.shrink_to_fit();
    }

    pub fn from_cols_and_attr(cols: &'u Columns, attr: &'u Vec<Attributes>, maps: &Maps<'u>) -> Res<Self> {
        let mut this = Self::new();
        attr.iter().for_each(|f| {
            this.add_tagfile(f);
        });
        cols.map.iter().try_for_each(|(fid, tid)| -> Res<()> {
            let f = maps.fids().by_uid(*fid)?;
            let t = maps.tids().by_uid(*tid)?;
            this.add_filetag(f, t);
            Ok(())
        })?;
        this.shrink_to_fit();
        Ok(this)
    }

    /// Add a filetag found in the database to the Diff
    pub fn add_filetag(&mut self, file: &'u str, tag: &'u str) {
        self.filetags.ls().insert((file, tag));
        self.files.ls().insert(file);
        self.tags.ls().insert(tag);
    }

    /// Add a tagfile found in the filesystem to the Diff
    pub fn add_tagfile(&mut self, file: &'u Attributes) {
        self.filetags.rs().extend(file.mapping());
        self.files.rs().insert(file.path_str());
        self.tags.rs().extend(file.iter().map(|tag| tag.as_str()));
    }

    /// Generate a 'diff' of attrs in the db and fs
    pub fn file_diff(&self) -> (DiffedStrIds, DiffedStrIds) {
        self.files.diff()
    }

    /// Generate a 'diff' of tags in the db and fs
    pub fn tag_diff(&self) -> (DiffedStrIds, DiffedStrIds) {
        self.tags.diff()
    }

    /// Generate a 'diff' of file/tag pairs in the db and fs
    pub fn filetag_diff(&self) -> (DiffedFileTags, DiffedFileTags) {
        self.filetags.diff()
    }
}
