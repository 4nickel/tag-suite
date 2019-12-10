pub mod state;

pub mod import {
    pub use super::super::import::*;
    pub use crate::{
        app::data::query::{self, export::*},
        db::export::*,
        model::export::*,
    };
}

pub mod export {
    pub use super::api::*;
    pub use super::state::*;
    pub use super::statistics::*;
}
pub use export::*;

pub mod statistics {

    use super::import::*;
    use crate::model::tag;

    pub struct Slot {
        id: Tid,
        occurrences: usize,
        pairings: usize,
    }

    impl Slot {
        pub fn new(id: Tid) -> Self {
            Self {
                id,
                occurrences: 1,
                pairings: 0,
            }
        }
        pub fn id(&self) -> Tid {
            self.id
        }
        pub fn occurrences(&self) -> usize {
            self.occurrences
        }
        pub fn bump_occurrence(&mut self) {
            self.occurrences += 1;
        }
        pub fn bump_pairing(&mut self) {
            self.pairings += 1;
        }
    }

    fn slots_pairs_and_assoc(
        rows: Vec<Ids>,
    ) -> (HashMap<Tid, Slot>, Vec<Ids>, HashMap<Fid, HashSet<Tid>>) {
        let mut pairs = Vec::new();
        let mut assoc: HashMap<Fid, HashSet<Tid>> = HashMap::new();
        let mut slots: HashMap<Tid, Slot> = HashMap::new();

        for (fid, tid) in rows {
            let tags = assoc.get_mut(&fid);
            match tags {
                Some(set) => {
                    /*
                     * Create pairs from this tag and the existing set of tags.
                     * Then add this tag to the set.
                     */
                    for t in set.iter() {
                        pairs.push((tid, *t));
                    }
                    set.insert(tid);
                }
                None => {
                    /*
                     * Create a new set of tags to keep track of this file.
                     */
                    let mut set = HashSet::new();
                    set.insert(tid);
                    assoc.insert(fid, set);
                }
            }
            if let Some(slot) = slots.get_mut(&tid) {
                slot.bump_occurrence();
            } else {
                slots.insert(tid, Slot::new(tid));
            }
        }
        (slots, pairs, assoc)
    }

    fn count_pairs(slots: &mut HashMap<Tid, Slot>, pairs: Vec<Ids>) -> HashMap<Ids, usize> {
        let mut count: HashMap<Ids, usize> = HashMap::new();
        for (lhs, rhs) in pairs {
            let n = *count.get(&(lhs, rhs)).unwrap_or(&0);
            count.insert((lhs, rhs), n + 1);
            count.insert((rhs, lhs), n + 1);
            slots.get_mut(&lhs).unwrap().bump_pairing();
            slots.get_mut(&rhs).unwrap().bump_pairing();
        }
        count
    }

    pub struct Statistics {
        pub count: HashMap<Ids, usize>,
        pub slots: HashMap<Tid, Slot>,
    }

    impl Statistics {
        pub fn from_filetags(filetags: Vec<Ids>) -> Self {
            let (mut slots, pairs, _) = slots_pairs_and_assoc(filetags);
            let count = count_pairs(&mut slots, pairs);
            Self { count, slots }
        }
        pub fn occurrences(&self, id: Tid) -> usize {
            self.slots.get(&id).map(|s| s.occurrences).unwrap_or(0usize)
        }
        pub fn iter_pairs_with_names<'q>(
            &'q self,
            names: &'q HashMap<Tid, String>,
        ) -> impl Iterator<Item = (tag::Borrow<'q>, tag::Borrow<'q>, usize)> + 'q {
            self.count.iter().map(move |((lid, rid), n)| {
                let lname = names.get(lid).unwrap();
                let rname = names.get(rid).unwrap();
                let l = tag::Borrow {
                    id: *lid,
                    name: lname,
                };
                let r = tag::Borrow {
                    id: *rid,
                    name: rname,
                };
                (l, r, *n)
            })
        }
        pub fn sorted_pairs_with_names<'q>(
            &'q self,
            names: &'q HashMap<Tid, String>,
        ) -> Vec<(tag::Borrow<'q>, tag::Borrow<'q>, usize)> {
            use std::cmp::Ordering;

            let mut items = self.iter_pairs_with_names(names).collect::<Vec<_>>();
            items.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));
            items.sort_by(|a, b| a.0.name.partial_cmp(&b.0.name).unwrap_or(Ordering::Equal));
            items
        }
    }
}

pub mod api {

    use super::{export::*, import::*};

    pub fn query_all_filetags(c: &db::Connection) -> Res<Vec<Ids>> {
        let rows: Vec<Ids> = file_tags::table
            .select((file_tags::file_id, file_tags::tag_id))
            .load(c.get())?;
        Ok(rows)
    }

    /// Return a list of all tag names and ids
    pub fn query_all_tags(c: &db::Connection) -> Res<Vec<TCol>> {
        Ok(tags::table.select((tags::id, tags::name)).load(c.get())?)
    }

    /// Return a list of all tag names and ids
    pub fn query_all_tags_mapped(c: &db::Connection) -> Res<HashMap<Tid, String>> {
        Ok(query_all_tags(c)?.into_iter().collect())
    }

    /// Sort a vector of tags lexically
    pub fn sort_tags_lexically(mut tags: Vec<TCol>) -> Vec<TCol> {
        use std::cmp::Ordering;
        tags.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        tags
    }

    pub fn collect_to_string(tags: Vec<TCol>) -> String {
        use crate::app::attr;
        tags.into_iter().fold(String::new(), |mut buf, (_, name)| {
            if let Some(_spooky) = attr::api::ghostbuster(name.as_str()) {
                buf.push_str(&name);
                buf.push('\n');
            }
            buf
        })
    }

    pub fn query_tag_statistics(c: &db::Connection) -> Res<Statistics> {
        let filetags = query_all_filetags(c)?;
        Ok(Statistics::from_filetags(filetags))
    }
}
