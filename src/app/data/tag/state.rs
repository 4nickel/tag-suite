use super::import::*;

pub enum Results<'q> {
    Unmapped(Unmapped),
    Unassociated(Unassociated),
    Tagged(Tagged<'q>),
}

/// Tagged Query Data
///
/// Takes Piped data and generates an
/// additional List of (filtered) Tag IDs.
pub struct Tagged<'q> {
    pub maps: OwnedMaps<'q>,
    pub fids: Vec<Fid>,
    pub tids: Vec<Tid>,
}

impl<'q> Tagged<'q> {
    pub fn from_piped(piped: Piped<'q>, dbq: &DatabaseQuery) -> Res<Self> {
        let tids: Vec<Fid> = query::api::query_tags_by_file_ids(&piped.fids, &dbq.api.connection)?;
        Ok(Self {
            maps: piped.maps,
            fids: piped.fids,
            tids,
        })
    }
    pub fn from_filtered(filtered: Filtered<'q>, dbq: &DatabaseQuery) -> Res<Self> {
        let tids: Vec<Fid> =
            query::api::query_tags_by_file_ids(&filtered.fids, &dbq.api.connection)?;
        Ok(Self {
            maps: filtered.maps,
            fids: filtered.fids,
            tids,
        })
    }
}

impl<'q> Viewable<'q> for Tagged<'q> {
    fn maps(&'q self) -> &'q Maps<'q> {
        self.maps.maps()
    }
    fn tag_view(&'q self) -> Box<dyn Iterator<Item = Fid> + 'q> {
        box self.tids.iter().map(|e| *e)
    }
    fn file_view(&'q self) -> Box<dyn Iterator<Item = Fid> + 'q> {
        box self.fids.iter().map(|e| *e)
    }
    fn file_count(&self) -> usize {
        self.maps.fids().len()
    }
}
