use super::import::*;
use crate::{app::attr::{File as Attributes}, util::collections};
use crate::{model::{file, tag}};

pub type DiffedFiles<'d> = collections::Diffed<'d, file::Ident<'d>>;
pub type DiffedTags<'d> = collections::Diffed<'d, tag::Ident<'d>>;
pub type DiffedFileTags<'d> = collections::Diffed<'d, (file::Ident<'d>, tag::Ident<'d>)>;

/// Sets of found tags/attrs in the
/// filesystem and database. Used to
/// generate 'diffs', which we then
/// use to create the insert and delete
/// statements.
pub struct Diff<'u> {
    files: collections::Diff<file::Ident<'u>>,
    tags: collections::Diff<tag::Ident<'u>>,
    filetags: collections::Diff<(file::Ident<'u>, tag::Ident<'u>)>,
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
        cols.filetag_iter().try_for_each(|(fid, tid)| -> Res<()> {
            let f = maps.fids().by_uid(fid)?;
            let t = maps.tids().by_uid(tid)?;
            this.add_filetag(f.ident(), t.ident());
            Ok(())
        })?;
        this.shrink_to_fit();
        Ok(this)
    }

    /// Add a filetag found in the database to the Diff
    pub fn add_filetag(&mut self, file: file::Ident<'u>, tag: tag::Ident<'u>) {
        self.files.ls().insert(file);
        self.tags.ls().insert(tag);
        self.filetags.ls().insert((file, tag));
    }

    /// Add a tagfile found in the filesystem to the Diff
    pub fn add_tagfile(&mut self, file: &'u Attributes) {
        self.filetags.rs().extend(file.mapping());
        self.files.rs().insert(file.ident());
        self.tags.rs().extend(file.iter().map(|tag| tag.ident()));
    }

    /// Generate a 'diff' of attrs in the db and fs
    pub fn file_diff(&self) -> (DiffedFiles, DiffedFiles) {
        self.files.diff()
    }

    /// Generate a 'diff' of tags in the db and fs
    pub fn tag_diff(&self) -> (DiffedTags, DiffedTags) {
        self.tags.diff()
    }

    /// Generate a 'diff' of file/tag pairs in the db and fs
    pub fn filetag_diff(&self) -> (DiffedFileTags, DiffedFileTags) {
        self.filetags.diff()
    }
}
