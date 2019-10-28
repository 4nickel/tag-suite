use super::{import::*, iter};
use crate::{util::collections, expression::Parameters, app::attr::Tag};
use owning_ref::OwningHandle;

pub type ManyToMany = collections::ManyToMany<Fid, Tid>;
pub type OneToOne<'q> = collections::OneToOne<Uid, &'q str>;

const TAGS_LEN: &'static str = "tags.len";
const PATH_LEN: &'static str = "path.len";
const FILE_ID: &'static str = "file.id";

/// A lot of our types have underlying
/// columns. We implement this trait to reduce
/// boilerplate at the cost of having to box
/// the iterator, which doesn't matter at all.
pub trait Columnar {
    fn files(&self) -> &Vec<(Fid, String)>;
    fn tags(&self) -> &Vec<(Tid, String)>;
    fn filetags(&self) -> &Vec<Ids>;
    /// Iterate the Files column
    fn file_iter<'q>(&'q self) -> Box<dyn Iterator<Item=(Fid, &'q str)> + 'q> {
        box self.files().iter().map(|e| (e.0, e.1.as_str()))
    }
    /// Iterate the Tags column
    fn tag_iter<'q>(&'q self) -> Box<dyn Iterator<Item=(Tid, &'q str)> + 'q> {
        box self.tags().iter().map(|e| (e.0, e.1.as_str()))
    }
    fn file_count(&self) -> usize { self.files().len() }
    fn tag_count(&self) -> usize { self.tags().len() }
}

/// This is the raw queried information.
/// The lifetime 'q in this module refers to
/// the lifetime of this data.
pub struct Columns {
    pub fcol: Vec<(Fid, String)>,
    pub tcol: Vec<(Tid, String)>,
    pub map: Vec<(Fid, Tid)>,
}

impl Columns {
    /// Iterate the Files column
    pub fn file_iter<'q>(&'q self) -> impl Iterator<Item=(Fid, &'q str)> {
        self.fcol.iter().map(|e| (e.0, e.1.as_str()))
    }
    /// Iterate the Tags column
    pub fn tag_iter<'q>(&'q self) -> impl Iterator<Item=(Tid, &'q str)> {
        self.tcol.iter().map(|e| (e.0, e.1.as_str()))
    }
    /// Shrink the internal buffers to fit
    pub fn shrink_to_fit(&mut self) {
        self.fcol.shrink_to_fit();
        self.tcol.shrink_to_fit();
        self.map.shrink_to_fit();
    }
}

impl Columnar for Columns {
    fn files(&self) -> &Vec<(Fid, String)> { &self.fcol }
    fn tags(&self) -> &Vec<(Tid, String)> { &self.tcol }
    fn filetags(&self) -> &Vec<Ids> { &self.map }
}

pub trait Viewable<'q> {
    fn maps(&'q self) -> &'q Maps<'q>;
    fn tag_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q>;
    fn file_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q>;
    fn file_iter(&'q self) -> Box<dyn Iterator<Item=(Fid, &'q str)> + 'q> {
        box MapFileIter::from_iter(&self.maps(), self.file_view()).map(|f| (f.id(), f.path()))
    }
    fn file_view_iter(&'q self) -> Box<dyn Iterator<Item=FileView<'q>> + 'q> {
        box MapFileIter::from_iter(&self.maps(), self.file_view())
    }
    fn tag_iter(&'q self) -> Box<dyn Iterator<Item=(Tid, &'q str)> + 'q> {
        box MapTagIter::from_iter(&self.maps(), self.tag_view()).map(|t| (t.id(), t.name()))
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
    fids: OneToOne<'q>,
    tids: OneToOne<'q>,
    mtom: ManyToMany,
}

impl<'q> Maps<'q> {

    /// Create a new instance with the given capacity
    /// preallocated along all maps
    pub fn new() -> Self {
        Self {
            fids: OneToOne::new(),
            tids: OneToOne::new(),
            mtom: ManyToMany::new(),
        }
    }

    pub fn from_cols_and_mtom(cols: &'q Columns, mtom: ManyToMany) -> Self {
        let mut this = Self {
            tids: OneToOne::with_capacity(cols.fcol.len()),
            fids: OneToOne::with_capacity(cols.tcol.len()),
            mtom: mtom,
        };
        cols.file_iter().for_each(|(id, path)| {
            this.add_file(id, path);
        });
        cols.tag_iter().for_each(|(id, name)| {
            this.add_tag(id, name);
        });
        this.shrink_to_fit();
        this
    }

    pub fn shrink_to_fit(&mut self) {
        self.fids.shrink_to_fit();
        self.tids.shrink_to_fit();
        self.mtom.shrink_to_fit();
    }

    pub fn fids(&self) -> &OneToOne<'q> { &self.fids }
    pub fn tids(&self) -> &OneToOne<'q> { &self.tids }
    pub fn mtom(&self) -> &ManyToMany { &self.mtom }

    /// Insert a file into the underlying maps
    #[inline(always)]
    pub fn add_file(&mut self, id: Fid, path: &'q str) {
        self.fids.map(id, path);
    }

    /// Insert a tag into the underlying maps
    #[inline(always)]
    pub fn add_tag(&mut self, id: Tid, name: &'q str) {
        self.tids.map(id, name);
    }

    /// Insert a pair of file and tag ids in to the underlying maps
    #[inline(always)]
    pub fn add_ids(&mut self, fid: Fid, tid: Tid) {
        self.mtom.map(fid, tid);
    }

    /// Add a file/tag mapping
    pub fn add_filetag(&mut self, fid: Fid, file: &'q str, tid: Tid, tag: &'q str) {
        self.add_file(fid, file);
        self.add_tag(tid, tag);
        self.add_ids(fid, tid);
    }

    /// View a file by id
    pub fn file(&'q self, id: Fid) -> Res<FileView<'q>> {
        let path = self.fids.by_uid(id)?;
        let tids = self.mtom.get_rs(id)?;
        Ok(FileView { id, maps: &self, path, tids })
    }

    /// View a tag by id
    pub fn tag(&'q self, id: Tid) -> Res<TagView<'q>> {
        let name = self.tids.by_uid(id)?;
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
    pub fn new(cols: Columns, mtom: ManyToMany) -> Self {
        Self {
            own: OwningHandle::new_with_fn(box cols, move |cols_p| {
                unsafe { box Maps::from_cols_and_mtom(&(*cols_p), mtom) }
            })
        }
    }
    #[inline(always)] pub fn fids<'a>(&'a self) -> &'a OneToOne<'q> { &self.own.fids }
    #[inline(always)] pub fn tids<'a>(&'a self) -> &'a OneToOne<'q> { &self.own.tids }
    #[inline(always)] pub fn mtom<'a>(&'a self) -> &'a ManyToMany { &self.own.mtom }
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
    path: &'q str,
    tids: &'q HashSet<Tid>,
    maps: &'q Maps<'q>,
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
    pub fn tag_set(&self) -> HashSet<Tag> {
        self.iter().map(|t| { Tag::new(t.name()).unwrap() }).collect()
    }

    /// Return an iterator over this files tags
    pub fn iter(&self) -> iter::MapTagIter<'q> {
        iter::MapTagIter::from_iter(self.maps, self.tids.iter().map(|t| *t))
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
}
