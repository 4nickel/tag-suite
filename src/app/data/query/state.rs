use super::{import::*, api, error::{Error as E}, maps::*};
use crate::{app::{meta::config, data::DatabaseLayer}, model::*};

/// Raw Query Data
///
/// This is the data our query dsl routine
/// returns. Pairs of Attr and Tag IDs each
/// representing a single AttrTag.
pub struct Raw {
    pub data: Vec<Ids>,
}

impl Raw {
    pub fn from_query(dbq: &DatabaseQuery) -> Res<Self> {
        let data = match &dbq.pipeline.query {
            Some(query) => api::query_dsl(&query, &dbq.api.connection)?,
            None => api::query_all(&dbq.api.connection)?
        };
        Ok(Self { data })
    }
}

/// Unassociated Query Data
///
/// Takes the Raw data and queries attr and
/// tag names. This State is only used when
/// we do *not* need to generate maps and are
/// just trying to get file/tag names to the
/// output asap.
pub struct Unassociated {
    pub cols: Columns,
}

impl Unassociated {
    pub fn from_raw(raw: Raw, dbq: &DatabaseQuery) -> Res<Self> {
        let cols = api::query_unassociated(raw.data, &dbq.api.connection)?;
        Ok(Self { cols })
    }
}

impl Columnar for Unassociated {
    fn files(&self) -> &Vec<FCol> { self.cols.files() }
    fn tags(&self) -> &Vec<TCol> { self.cols.tags() }
    fn filetags(&self) -> &Vec<Ids> { self.cols.filetags() }
}

/// Unmapped Query Data
///
/// Takes the Raw data and associates it
/// with attr and tag names, as well as
/// generating the maps, which define our
/// Tag-to-Attr ManyToMany.
pub struct Unmapped {
    pub cols: Columns,
    pub asso: ManyToManyIds,
}

impl Unmapped {
    pub fn from_raw(raw: Raw, dbq: &DatabaseQuery) -> Res<Self> {
        let (cols, asso) = api::query_associated(raw.data, &dbq.api.connection)?;
        Ok(Self { cols, asso })
    }
}

impl Columnar for Unmapped {
    fn files(&self) -> &Vec<FCol> { self.cols.files() }
    fn tags(&self) -> &Vec<TCol> { self.cols.tags() }
    fn filetags(&self) -> &Vec<Ids> { self.cols.filetags() }
}

/// Mapped Query Data
///
/// Takes the Unmapped data and generates
/// the final mappings. These maps provide
/// an efficient and comprehensive way of
/// viewing the data and it's ManyToMany.
pub struct Mapped<'q> {
    pub maps: OwnedMaps<'q>,
}

impl<'q> Mapped<'q> {
    pub fn from_rows(cols: Unmapped) -> Self {
        Self { maps: OwnedMaps::new(cols.cols, cols.asso) }
    }
}

impl<'q> Viewable<'q> for Mapped<'q> {
    fn maps(&'q self) -> &'q Maps<'q> { self.maps.maps() }
    fn tag_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { self.maps.tag_view() }
    fn file_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { self.maps.file_view() }
}

/// Filtered Query Data
///
/// Takes the Mapped data filters it, storing
/// the filtered attrs by ID.
pub struct Filtered<'q> {
    pub maps: OwnedMaps<'q>,
    pub fids: Vec<Fid>
}

impl<'q> Filtered<'q> {
    pub fn from_mapped(mapped: Mapped<'q>, dbq: &DatabaseQuery) -> Res<Self> {
        use super::filter::*;
        let ast = dbq.pipeline.get_filter()
            .as_ref().map(|e| e.as_ast())
            .ok_or(E::WrongState {
                state: "mapped (no filter)".into(),
                operation: "filter".into(),
            })?;
        let mut fids =
            DslFilter::new(mapped.maps.inner(), ast)
                .filter(mapped.maps.fids().iter().map(|e| e.0));
        fids.shrink_to_fit();
        Ok(Self { maps: mapped.maps, fids })
    }
}

impl<'q> Viewable<'q> for Filtered<'q> {
    fn maps(&'q self) -> &'q Maps<'q> { self.maps.maps() }
    fn tag_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { self.maps.tag_view() }
    fn file_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { box self.fids.iter().map(|e| *e) }
    fn file_count(&self) -> usize { self.maps.fids().len() }
}

/// Piped Query Data
///
/// Takes the Mapped data filters it, storing
/// the filtered attrs by ID.
pub struct Piped<'q> {
    pub maps: OwnedMaps<'q>,
    pub fids: Vec<Fid>
}

impl<'q> Piped<'q> {
    pub fn from_filtered(filtered: Filtered<'q>, dbq: &DatabaseQuery) -> Res<Self> {
        use super::filter::*;
        let pipe = dbq.pipeline.get_pipe().as_ref()
            .ok_or(E::WrongState {
                state: "filtered (no pipe)".into(),
                operation: "pipe".into(),
            })?;
        let mut fids =
            PipeFilter::new(filtered.maps.inner(), pipe)
                .filter(filtered.fids.iter().map(|e| *e));
        fids.shrink_to_fit();
        Ok(Self { maps: filtered.maps, fids })
    }
    pub fn from_mapped(mapped: Mapped<'q>, dbq: &DatabaseQuery) -> Res<Self> {
        use super::filter::*;
        let pipe = dbq.pipeline.get_pipe().as_ref()
            .ok_or(E::WrongState {
                state: "mapped (no pipe)".into(),
                operation: "pipe".into(),
            })?;
        let mut fids =
            PipeFilter::new(mapped.maps.inner(), pipe)
                .filter(mapped.maps.fids().iter().map(|e| e.0));
        fids.shrink_to_fit();
        Ok(Self { maps: mapped.maps, fids })
    }
}

impl<'q> Viewable<'q> for Piped<'q> {
    fn maps(&'q self) -> &'q Maps<'q> { self.maps.maps() }
    fn tag_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { self.maps.tag_view() }
    fn file_view(&'q self) -> Box<dyn Iterator<Item=Fid> + 'q> { box self.fids.iter().map(|e| *e) }
    fn file_count(&self) -> usize { self.maps.fids().len() }
}

/// Query results come in different
/// shapes, depending on how strict our
/// requirements and forcings are.
pub enum Results<'q> {
    Unmapped(Unmapped),
    Unassociated(Unassociated),
    Mapped(Mapped<'q>),
    Filtered(Filtered<'q>),
    Piped(Piped<'q>),
}

impl<'q> Results<'q> {

    pub fn name(&self) -> &'static str {
        match &self {
            Self::Unassociated(_inner) => "Unassociated",
            Self::Unmapped(_inner) => "Unmapped",
            Self::Mapped(_inner) => "Mapped",
            Self::Filtered(_inner) => "Filtered",
            Self::Piped(_inner) => "Piped",
        }
    }

    // TODO: get rid of all these switched.
    // flesh out the columnar and viewable traits
    // and box the data instead of the iterators

    pub fn row_count(&self) -> usize {
        match &self {
            Self::Unassociated(inner) => inner.file_count(),
            Self::Unmapped(inner) => inner.file_count(),
            Self::Mapped(inner) => inner.file_count(),
            Self::Filtered(inner) => inner.file_count(),
            Self::Piped(inner) => inner.file_count(),
        }
    }

    pub fn file_count(&self) -> usize {
        match &self {
            Self::Unassociated(inner) => inner.file_count(),
            Self::Unmapped(inner) => inner.file_count(),
            Self::Mapped(inner) => inner.file_count(),
            Self::Filtered(inner) => inner.file_count(),
            Self::Piped(inner) => inner.file_count(),
        }
    }

    pub fn file_iter(&'q self) -> Result<Box<dyn Iterator<Item=file::Borrow<'q>> + 'q>, E> {
        match &self {
            Self::Unassociated(inner) => Ok(box inner.file_iter()),
            Self::Unmapped(inner) => Ok(box inner.file_iter()),
            Self::Mapped(inner) => Ok(box inner.file_iter()),
            Self::Filtered(inner) => Ok(box inner.file_iter()),
            Self::Piped(inner) => Ok(box inner.file_iter()),
        }
    }

    pub fn tag_iter(&'q self) -> Result<Box<dyn Iterator<Item=tag::Borrow<'q>> + 'q>, E> {
        match &self {
            Self::Unassociated(inner) => Ok(box inner.tag_iter()),
            Self::Unmapped(inner) => Ok(box inner.tag_iter()),
            Self::Mapped(inner) => Ok(box inner.tag_iter()),
            _ => Err(E::WrongState { state: self.name().into(), operation: "tag_iter()".into() }.into()),
        }
    }

    pub fn file_view_iter(&'q self) -> Result<Box<dyn Iterator<Item=FileView<'q>> + 'q>, E> {
        match &self {
            Self::Mapped(inner) => Ok(box inner.file_view_iter()),
            Self::Filtered(inner) => Ok(box inner.file_view_iter()),
            _ => Err(E::WrongState { state: self.name().into(), operation: "file_view_iter()".into() }.into()),
        }
    }

    pub fn tag_view_iter(&'q self) -> Result<Box<dyn Iterator<Item=TagView<'q>> + 'q>, E> {
        match &self {
            Self::Mapped(inner) => Ok(box inner.tag_view_iter()),
            _ => Err(E::WrongState { state: self.name().into(), operation: "tag_view_iter()".into() }.into()),
        }
    }
}

/// Holds the Query State.
/// Used to drive queries to completion.
pub enum State<'q> {
    Incomplete(Progress<'q>),
    Done(Results<'q>),
}

impl<'q> State<'q> {
    /// Process this query.
    pub fn process(mut self, dbq: &DatabaseQuery) -> Res<Results<'q>> {
        profile!("drive", {
            while let State::Incomplete(progress) = self {
                self = progress.drive(dbq)?;
            }
        });
        if let State::Done(results) = self {
            Ok(results)
        } else {
            panic!("query is in a broken state")
        }
    }
}

/// Encodes Query Progress.
pub enum Progress<'q> {
    Init,
    Raw(Raw),
    Unassociated(Unassociated),
    Unmapped(Unmapped),
    Mapped(Mapped<'q>),
    Filtered(Filtered<'q>),
    Piped(Piped<'q>),
}

impl<'q> Progress<'q> {
    /// Drive query progress.
    fn drive(self, dbq: &DatabaseQuery) -> Res<State<'q>> {
        use super::*;
        match self {
            Self::Init => {
                profile!("init", { Ok(State::Incomplete(Progress::Raw(Raw::from_query(dbq)?))) })
            }
            Self::Raw(inner) => {
                profile!("raw", { if dbq.forcings.has_none() {
                    Ok(State::Incomplete(Progress::Unassociated(Unassociated::from_raw(inner, dbq)?)))
                } else {
                    Ok(State::Incomplete(Progress::Unmapped(Unmapped::from_raw(inner, dbq)?)))
                } })
            }
            Self::Unassociated(inner) => {
                Ok(State::Done(Results::Unassociated(inner)))
            }
            Self::Unmapped(inner) => {
                profile!("unmapped", { if dbq.forcings.has_none() {
                    Ok(State::Done(Results::Unmapped(inner)))
                } else {
                    Ok(State::Incomplete(Progress::Mapped(Mapped::from_rows(inner))))
                } })
            }
            Self::Mapped(inner) => {
                profile!("mapped", { if dbq.forcings.has_filtered() {
                    Ok(State::Incomplete(Progress::Filtered(Filtered::from_mapped(inner, dbq)?)))
                } else if dbq.forcings.has_piped() {
                    Ok(State::Incomplete(Progress::Piped(Piped::from_mapped(inner, dbq)?)))
                } else {
                    Ok(State::Done(Results::Mapped(inner)))
                } })
            }
            Self::Filtered(inner) => {
                profile!("filtered", { if dbq.forcings.has_piped() {
                    Ok(State::Done(Results::Piped(Piped::from_filtered(inner, dbq)?)))
                } else {
                    Ok(State::Done(Results::Filtered(inner)))
                } })
            }
            Self::Piped(inner) => {
                profile!("piped", {
                    Ok(State::Done(Results::Piped(inner)))
                })
            }
        }
    }
}

/// A concrete Query against a Database
/// with Forcings.
pub struct DatabaseQuery<'e> {
    pub api: &'e DatabaseLayer,
    pub pipeline: &'e Pipeline,
    pub forcings: Forcings,
}

/// A single concrete DatabaseQuery and
/// all the state needed to drive it.
pub struct Query<'e, 'q> {
    dbq: DatabaseQuery<'e>,
    state: State<'q>,
}

impl<'e, 'q> Query<'e, 'q> {

    /// Create a new query.
    pub fn new(pipeline: &'e Pipeline, forcings: Forcings, api: &'e DatabaseLayer) -> Self {
        let forcings = forcings.combine(pipeline.forcings());
        Self {
            dbq: DatabaseQuery { api, pipeline, forcings },
            state: State::Incomplete(Progress::Init)
        }
    }

    /// Run this query.
    #[inline(always)]
    pub fn execute(self) -> Res<Results<'q>> {
        self.state.process(&self.dbq)
    }
}

/// An entire query-pipeline.
#[derive(Debug, Clone)]
pub struct Pipeline {
    query: Option<CompiledExpression>,
    filter: Option<CompiledExpression>,
    pipe: Option<String>,
}

impl Pipeline {

    /// Create a new query in it's initial state.
    pub fn new(query: Option<CompiledExpression>, filter: Option<CompiledExpression>, pipe: Option<String>) -> Self {
        Self { query, filter, pipe }
    }

    /// Create a new pipeline from buffers.
    pub fn from_pipeline(pipeline: config::PipelineBuf) -> Res<Self> {
        Self::from_strings(pipeline.query, pipeline.filter, pipeline.pipe)
    }

    /// Create a new pipeline from strings.
    pub fn from_strings(query: Option<String>, filter: Option<String>, pipe: Option<String>) -> Res<Self> {
        let (query, filter) = CompiledExpression::compile(query, filter)?;
        Ok(Self { query, filter, pipe })
    }

    /// Get this pipelines minimum forcings.
    pub fn forcings(&self) -> Forcings {
        let mut f = Forcings::new();
        if let Some(_) = self.filter { f = f.mapped().filtered() }
        if let Some(_) = self.pipe   { f = f.mapped().piped() }
        f
    }

    /// Get this pipelines inner query expression.
    #[inline(always)]
    pub fn get_query(&self) -> &Option<CompiledExpression> {
        &self.query
    }

    /// Get this pipelines inner filter expression.
    #[inline(always)]
    pub fn get_filter(&self) -> &Option<CompiledExpression> {
        &self.filter
    }

    /// Get this pipelines inner pipe expression.
    #[inline(always)]
    pub fn get_pipe(&self) -> &Option<String> {
        &self.pipe
    }

    /// Process this query with forcings.
    pub fn forced_query<'e, 'q>(&'e self, forcings: Forcings, api: &'e DatabaseLayer) -> Query<'e, 'q> {
        Query::new(&self, forcings, api)
    }

    /// Process this query without forcings.
    pub fn query<'e, 'q>(&'e self, api: &'e DatabaseLayer) -> Query<'e, 'q> {
        Query::new(&self, Forcings::new(), api)
    }
}
