use super::{import::*, export::*};

/// An iterator providing a borrowd view
/// into our mapped data.
pub struct MapFileIter<'q> {
    data: &'q Maps<'q>,
    iter: Box<dyn Iterator<Item=Fid> + 'q>,
}

impl<'q> MapFileIter<'q> {

    pub fn new(data: &'q Maps) -> Self {
        Self {
            data: data,
            iter: box data.fids().iter().map(|e| e.0)
        }
    }

    pub fn from_iter<I: Iterator<Item=Fid> + 'q>(data: &'q Maps, iter: I) -> Self {
        Self { data: data, iter: box iter }
    }
}

impl<'q> Iterator for MapFileIter<'q> {
    type Item = FileView<'q>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|id| self.data.file(id).unwrap())
    }
}

/// An iterator providing a borrowd view
/// into our mapped data.
pub struct MapTagIter<'q> {
    data: &'q Maps<'q>,
    iter: Box<dyn Iterator<Item=Fid> + 'q>
}

impl<'q> MapTagIter<'q> {

    pub fn new(data: &'q Maps) -> Self {
        Self {
            data: data,
            iter: box data.tids().iter().map(|e| e.0)
        }
    }

    pub fn from_iter<I: Iterator<Item=Fid> + 'q>(data: &'q Maps, iter: I) -> Self {
        Self { data: data, iter: box iter }
    }
}

impl<'q> Iterator for MapTagIter<'q> {
    type Item = TagView<'q>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|id| self.data.tag(id).unwrap())
    }
}
