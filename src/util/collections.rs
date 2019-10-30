use super::import::*;
use std::collections::{hash_map::RandomState, hash_set::Difference};
use std::hash::Hash;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "unknown id: '{}'", id)]
        UnknownId { id: String, },
    }
}
use error::{Error as E};

/// Map Many Ids on the left-hand-side to Many Ids on the right-hand-side
pub struct ManyToMany<L, R>
where
    L: Hash + Eq,
    R: Hash + Eq
{
    ltor: HashMap<L, HashSet<R>>,
    rtol: HashMap<R, HashSet<L>>,
}

impl<L, R> ManyToMany<L, R>
where
    L: Hash + Eq + Copy,
    R: Hash + Eq + Copy
{
    pub fn new() -> Self {
        Self {
            ltor: HashMap::new(),
            rtol: HashMap::new(),
        }
    }
    pub fn from_maps(ltor: HashMap<L, HashSet<R>>, rtol: HashMap<R, HashSet<L>>) -> Self {
        Self { ltor, rtol }
    }
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            ltor: HashMap::with_capacity(cap),
            rtol: HashMap::with_capacity(cap),
        }
    }
    /// Map an int to another int
    pub fn map(&mut self, l: L, r: R) -> (bool, bool) {
        let lonce = if !self.ltor.contains_key(&l) {
            self.ltor.insert(l, HashSet::with_capacity(2)); true
        } else { false };
        let ronce = if !self.rtol.contains_key(&r) {
            self.rtol.insert(r, HashSet::with_capacity(2)); true
        } else { false };
        self.ltor.get_mut(&l).unwrap().insert(r);
        self.rtol.get_mut(&r).unwrap().insert(l);
        (lonce, ronce)
    }
    /// Return the number of items on the left-hand-side
    #[inline(always)] pub fn ls_len(&self) -> usize { self.ltor.len() }
    /// Return the number of items on the right-hand-side
    #[inline(always)] pub fn rs_len(&self) -> usize { self.rtol.len() }
    /// Return the set of items mapped to the given left-hand-side item
    #[inline(always)] pub fn get_rs(&self, l: L) -> Res<&HashSet<R>> { Ok(self.ltor.get(&l).unwrap()) }
    /// Return the set of items mapped to the given right-hand-side item
    #[inline(always)] pub fn get_ls(&self, r: R) -> Res<&HashSet<L>> { Ok(self.rtol.get(&r).unwrap()) }
    /// Shrink the internal buffers to fit
    pub fn shrink_to_fit(&mut self) {
        self.ltor.shrink_to_fit();
        self.rtol.shrink_to_fit();
    }
}

pub struct OneToOne<L, R>
where
    L: Hash + Eq,
    R: Hash + Eq
{
    itos: HashMap<L, R>,
    stoi: HashMap<R, L>,
}

impl<L, R> OneToOne<L, R>
where
    L: Hash + Eq + Copy,
    R: Hash + Eq + Copy
{
    pub fn new() -> Self {
        Self {
            itos: HashMap::new(),
            stoi: HashMap::new()
        }
    }
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            itos: HashMap::with_capacity(cap),
            stoi: HashMap::with_capacity(cap)
        }
    }
    /// Return an iterator over the int/str pairs
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=(L, R)> + 'a {
        self.itos.iter().map(|e| (*e.0, *e.1))
    }
    /// Return the number of contained items
    pub fn len(&self) -> usize {
        self.itos.len()
    }
    /// Return the str associated with an int
    pub fn by_uid(&self, id: L) -> Res<R> {
        self.itos.get(&id).map(|id| *id)
            .ok_or(E::UnknownId { id: "".into() }.into())
    }
    /// Return the int associated with an str
    pub fn by_alt(&self, id: R) -> Res<L> {
        self.stoi.get(&id).map(|id| *id)
            .ok_or(E::UnknownId { id: "".into() }.into())
    }
    /// Map an int to a str
    pub fn map(&mut self, l: L, r: R) {
        self.itos.insert(l, r);
        self.stoi.insert(r, l);
    }
    /// Shrink the internal buffers to fit
    pub fn shrink_to_fit(&mut self) {
        self.stoi.shrink_to_fit();
        self.itos.shrink_to_fit();
    }
}

pub trait HasId<T>
where
    T: Copy + Hash + Eq
{
    fn as_id(&self) -> T;
}

impl<T> HasId<T> for T
where
    T: Copy + Hash + Eq
{
    fn as_id(&self) -> T { *self }
}

pub struct OneToOneFat<L, LID, R, RID>
where
    L: HasId<LID>,
    R: HasId<RID>,
    RID: Copy + Hash + Eq,
    LID: Copy + Hash + Eq,
{
    itos: HashMap<LID, R>,
    stoi: HashMap<RID, L>,
}

impl<L, LID, R, RID> OneToOneFat<L, LID, R, RID>
where
    L: HasId<LID>,
    R: HasId<RID>,
    RID: Copy + Hash + Eq,
    LID: Copy + Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            itos: HashMap::new(),
            stoi: HashMap::new()
        }
    }
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            itos: HashMap::with_capacity(cap),
            stoi: HashMap::with_capacity(cap)
        }
    }
    /// Return an iterator over the int/str pairs
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=(LID, RID)> + 'a {
        self.itos.iter().map(|e| (*e.0, e.1.as_id()))
    }
    /// Return the number of contained items
    pub fn len(&self) -> usize {
        self.itos.len()
    }
    /// Return the str associated with an int
    pub fn by_uid(&self, id: LID) -> Res<&R> {
        self.itos.get(&id).ok_or(E::UnknownId { id: "".into() }.into())
    }
    /// Return the int associated with an str
    pub fn by_alt(&self, id: RID) -> Res<&L> {
        self.stoi.get(&id).ok_or(E::UnknownId { id: "".into() }.into())
    }
    /// Map an int to a str
    pub fn map(&mut self, l: L, r: R) {
        let lid = l.as_id();
        let rid = r.as_id();
        self.itos.insert(lid, r);
        self.stoi.insert(rid, l);
    }
    /// Shrink the internal buffers to fit
    pub fn shrink_to_fit(&mut self) {
        self.stoi.shrink_to_fit();
        self.itos.shrink_to_fit();
    }
}

pub type Diffed<'d, T> = Difference<'d, T, RandomState>;

/// Contains to sets of items for the purpose
/// of diffing them.
pub struct Diff<D>
where
    D: Hash + Eq,
{
    ls: HashSet<D>,
    rs: HashSet<D>,
}

impl<D> Diff<D>
where
    D: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            ls: HashSet::new(),
            rs: HashSet::new(),
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.ls.shrink_to_fit();
        self.rs.shrink_to_fit();
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            ls: HashSet::with_capacity(cap),
            rs: HashSet::with_capacity(cap),
        }
    }

    /// Get left-hand-side items. TODO: rename this
    pub fn ls(&mut self) -> &mut HashSet<D> { &mut self.ls }
    /// Get right-hand-side items. TODO: rename this
    pub fn rs(&mut self) -> &mut HashSet<D> { &mut self.rs }

    /// The the two internal sets symmetrically
    pub fn diff<'d>(&'d self) -> (Diffed<'d, D>, Diffed<'d, D>) {
        (self.ls.difference(&self.rs), self.rs.difference(&self.ls))
    }
}
