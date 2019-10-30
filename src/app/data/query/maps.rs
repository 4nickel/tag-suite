use super::{import::*, iter};
use crate::{
    model::{
        tag::{self, TagExt},
        file::{self, FileExt}
    },
    util::collections,
    expression::Parameters,
    app::attr::Tag
};
use owning_ref::OwningHandle;

pub type ManyToManyIds = collections::ManyToMany<Fid, Tid>;
pub type OneToOne<'q> = collections::OneToOne<Uid, &'q str>;
pub type OneToOneTid<'q> = collections::OneToOneFat<Uid, Uid, tag::Borrow<'q>, tag::Ident<'q>>;
pub type OneToOneFid<'q> = collections::OneToOneFat<Uid, Uid, file::Borrow<'q>, file::Ident<'q>>;

impl<'q> collections::HasId<tag::Ident<'q>> for &'q tag::Borrow<'q>
{ fn as_id(&self) -> tag::Ident<'q> { self.ident() } }

impl<'q> collections::HasId<tag::Ident<'q>> for tag::Borrow<'q>
{ fn as_id(&self) -> tag::Ident<'q> { self.ident() } }

impl<'q> collections::HasId<file::Ident<'q>> for &'q file::Borrow<'q>
{ fn as_id(&self) -> file::Ident<'q> { self.ident() } }

impl<'q> collections::HasId<file::Ident<'q>> for file::Borrow<'q>
{ fn as_id(&self) -> file::Ident<'q> { self.ident() } }

const TAGS_LEN: &'static str = "tags.len";
const PATH_LEN: &'static str = "path.len";
const FILE_ID: &'static str = "file.id";

/// A lot of our types have underlying
/// columns. We implement this trait to reduce
/// boilerplate at the cost of having to box
/// the iterator, which doesn't matter at all.
pub trait Columnar {
    fn files(&self) -> &Vec<FCol>;
    fn tags(&self) -> &Vec<TCol>;
    fn filetags(&self) -> &Vec<Ids>;
    /// Iterate the Files column
    fn file_iter<'q>(&'q self) -> Box<dyn Iterator<Item=file::Borrow<'q>> + 'q> {
        box self.files().iter().map(|row| row.borrow())
    }
    /// Iterate the Tags column
    fn tag_iter<'q>(&'q self) -> Box<dyn Iterator<Item=tag::Borrow<'q>> + 'q> {
        box self.tags().iter().map(|row| row.borrow())
    }
    /// Iterate the Maps
    fn filetag_iter<'q>(&'q self) -> Box<dyn Iterator<Item=(Fid, Tid)> + 'q> {
        box self.filetags().iter().map(|e| (e.0, e.1))
    }
    fn file_count(&self) -> usize { self.files().len() }
    fn tag_count(&self) -> usize { self.tags().len() }
}

/// This is the raw queried information.
/// The lifetime 'q in this module refers to
/// the lifetime of this data.
pub struct Columns {
    fcol: Vec<FCol>,
    tcol: Vec<TCol>,
    map: Vec<Ids>,
}

impl Columns {
    pub fn from_cols(fcol: Vec<FCol>, tcol: Vec<TCol>, map: Vec<Ids>) -> Self {
        Self { fcol, tcol, map }
    }
    /// Shrink the internal buffers to fit
    pub fn shrink_to_fit(&mut self) {
        self.fcol.shrink_to_fit();
        self.tcol.shrink_to_fit();
        self.map.shrink_to_fit();
    }
}

impl Columnar for Columns {
    fn files(&self) -> &Vec<FCol> { &self.fcol }
    fn tags(&self) -> &Vec<TCol> { &self.tcol }
    fn filetags(&self) -> &Vec<Ids> { &self.map }
}

pub trait Viewable<'q> {
    fn maps(&'q self) -> &'q Maps<'q>;
    fn tag_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q>;
    fn file_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q>;
    fn file_iter(&'q self) -> Box<dyn Iterator<Item=file::Borrow<'q>> + 'q> {
        box MapFileIter::from_iter(&self.maps(), self.file_view())
            .map(|f| f.as_borrow())
    }
    fn file_view_iter(&'q self) -> Box<dyn Iterator<Item=FileView<'q>> + 'q> {
        box MapFileIter::from_iter(&self.maps(), self.file_view())
    }
    fn tag_iter(&'q self) -> Box<dyn Iterator<Item=tag::Borrow<'q>> + 'q> {
        box MapTagIter::from_iter(&self.maps(), self.tag_view())
            .map(|t| t.as_borrow())
    }
    fn tag_view_iter(&'q self) -> Box<dyn Iterator<Item=TagView<'q>> + 'q> {
        box MapTagIter::from_iter(&self.maps(), self.tag_view())
    }
    fn file_count(&'q self) -> usize {
        self.maps().fids().len()
    }
    fn tag_count(&'q self) -> usize {
        self.maps().tids().len()
    }
}

/// Indexes the queried data.
/// Used mostly for updating and filtering.
/// Provides the mappings
pub struct Maps<'q> {
    fids: OneToOneFid<'q>,
    tids: OneToOneTid<'q>,
    mtom: ManyToManyIds,
}

impl<'q> Maps<'q> {

    /// Create a new instance with the given capacity
    /// preallocated along all maps
    pub fn new() -> Self {
        Self {
            fids: OneToOneFid::new(),
            tids: OneToOneTid::new(),
            mtom: ManyToManyIds::new(),
        }
    }

    pub fn from_cols_and_mtom(cols: &'q Columns, mtom: ManyToManyIds) -> Self {
        let mut this = Self {
            fids: OneToOneFid::with_capacity(cols.tcol.len()),
            tids: OneToOneTid::with_capacity(cols.fcol.len()),
            mtom: mtom,
        };
        cols.file_iter().for_each(|f| { this.add_file(f); });
        cols.tag_iter().for_each(|t| { this.add_tag(t); });
        this.shrink_to_fit();
        this
    }

    pub fn shrink_to_fit(&mut self) {
        self.fids.shrink_to_fit();
        self.tids.shrink_to_fit();
        self.mtom.shrink_to_fit();
    }

    pub fn fids(&self) -> &OneToOneFid<'q> { &self.fids }
    pub fn tids(&self) -> &OneToOneTid<'q> { &self.tids }
    pub fn mtom(&self) -> &ManyToManyIds { &self.mtom }

    /// Insert a file into the underlying maps
    #[inline(always)]
    pub fn add_file(&mut self, f: file::Borrow<'q>) {
        self.fids.map(f.id, f);
    }

    /// Insert a tag into the underlying maps
    #[inline(always)]
    pub fn add_tag(&mut self, t: tag::Borrow<'q>) {
        self.tids.map(t.id, t);
    }

    /// Insert a pair of file and tag ids in to the underlying maps
    #[inline(always)]
    pub fn add_ids(&mut self, fid: Fid, tid: Tid) {
        self.mtom.map(fid, tid);
    }

    /// Add a file/tag mapping
    pub fn add_filetag(&mut self, f: file::Borrow<'q>, t: tag::Borrow<'q>) {
        self.add_file(f);
        self.add_tag(t);
        self.add_ids(f.id, t.id);
    }

    /// View a file by id
    pub fn file(&'q self, id: Fid) -> Res<FileView<'q>> {
        let file = self.fids.by_uid(id)?;
        let tids = self.mtom.get_rs(id)?;
        Ok(FileView { id, maps: &self, path: file.path, kind: file.kind, tids })
    }

    /// View a tag by id
    pub fn tag(&'q self, id: Tid) -> Res<TagView<'q>> {
        let name = self.tids.by_uid(id)?.name;
        let fids = self.mtom.get_ls(id)?;
        Ok(TagView { id, maps: &self, name, fids })
    }
}

impl<'q> Viewable<'q> for Maps<'q> {
    fn maps(&'q self) -> &'q Maps<'q> { &self }
    fn tag_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { box self.tids.iter().map(|t| t.0) }
    fn file_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { box self.fids.iter().map(|f| f.0) }
}

use super::iter::MapFileIter;
impl<'q> IntoIterator for &'q Maps<'q> {
    type Item = FileView<'q>;
    type IntoIter = MapFileIter<'q>;

    fn into_iter(self) -> Self::IntoIter {
        MapFileIter::new(self)
    }
}

pub struct OwnedMaps<'q> {
    own: OwningHandle<Box<Columns>, Box<Maps<'q>>>
}

impl<'q> OwnedMaps<'q> {
    pub fn new(cols: Columns, mtom: ManyToManyIds) -> Self {
        Self {
            own: OwningHandle::new_with_fn(box cols, move |cols_p| {
                unsafe { box Maps::from_cols_and_mtom(&(*cols_p), mtom) }
            })
        }
    }
    #[inline(always)] pub fn fids<'a>(&'a self) -> &'a OneToOneFid<'q> { &self.own.fids }
    #[inline(always)] pub fn tids<'a>(&'a self) -> &'a OneToOneTid<'q> { &self.own.tids }
    #[inline(always)] pub fn mtom<'a>(&'a self) -> &'a ManyToManyIds { &self.own.mtom }
    #[inline(always)] pub fn inner<'a>(&'a self) -> &'a Maps<'q> { &self.own }
}

impl<'q> Viewable<'q> for OwnedMaps<'q> {
    fn maps(&'q self) -> &'q Maps<'q> { self.inner().maps() }
    fn tag_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { self.inner().tag_view() }
    fn file_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { self.inner().file_view() }
}

/// A borrowed view of a file and it's associated tags
pub struct FileView<'q> {
    id: Fid,
    kind: Kind,
    path: &'q str,
    tids: &'q HashSet<Tid>,
    maps: &'q Maps<'q>,
}

impl<'q> FileExt for FileView<'q> {
    fn id(&self) -> Tid { self.id }
    fn kind(&self) -> Kind { self.kind }
    fn path<'a>(&'a self) -> &'a str { self.path }
}

impl<'q> FileView<'q> {

    /// Return this files id
    #[inline(always)]
    pub fn id(&self) -> Fid {
        self.id
    }

    /// Return this files path
    #[inline(always)]
    pub fn path(&self) -> &'q str {
        self.path
    }

    /// Return this files associated tag ids
    #[inline(always)]
    pub fn tag_ids(&self) -> &'q HashSet<Tid> {
        self.tids
    }

    /// Collect this files associated tag set
    pub fn tag_set(&self) -> Res<HashSet<Tag>> {
        self.iter().try_fold(HashSet::new(), |mut set, t| {
            set.insert(Tag::new(t.name())?);
            Ok(set)
        })
    }

    /// Return an iterator over this files tags
    pub fn iter(&self) -> iter::MapTagIter<'q> {
        iter::MapTagIter::from_iter(self.maps, self.tids.iter().map(|t| *t))
    }

    /// Turn this view into the simpler Borrow
    pub fn as_borrow(&self) -> file::Borrow<'q> {
        file::Borrow {
            id: self.id,
            path: self.path,
            kind: self.kind,
        }
    }
}

/// A files parameters can be used as lhs in comparison filters
impl<'q> Parameters for FileView<'q> {
    fn parameters(&self) -> HashMap<&'static str, usize> {
        let mut map = HashMap::new();
        map.insert(TAGS_LEN, self.tids.len() - 1);
        map.insert(PATH_LEN, self.path.len());
        map.insert(FILE_ID, self.id() as usize);
        map
    }
}

/// A borrowed view of a tag and it's associated files
pub struct TagView<'q> {
    id: Tid,
    name: &'q str,
    fids: &'q HashSet<Fid>,
    maps: &'q Maps<'q>,
}

impl<'q> TagExt for TagView<'q> {
    fn id(&self) -> Tid { self.id }
    fn name<'a>(&'a self) -> &'a str { self.name }
}

impl<'q> TagView<'q> {

    /// Return this tags id
    #[inline(always)]
    pub fn id(&self) -> Tid {
        self.id
    }

    /// Return this tags name
    #[inline(always)]
    pub fn name(&self) -> &'q str {
        self.name
    }

    /// Return this tags associated files ids
    #[inline(always)]
    pub fn file_ids(&self) -> &'q HashSet<Fid> {
        self.fids
    }

    /// Return an iterator over this tags associated files
    #[inline(always)]
    pub fn iter(&self) -> iter::MapFileIter<'q> {
        iter::MapFileIter::from_iter(self.maps, self.fids.iter().map(|f| *f))
    }

    /// Turn this view into the simpler Borrow
    pub fn as_borrow(&self) -> tag::Borrow<'q> {
        tag::Borrow {
            id: self.id,
            name: self.name,
        }
    }
}
