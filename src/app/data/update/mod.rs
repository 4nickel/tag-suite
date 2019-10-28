pub mod scan;
pub mod state;
pub mod diff;

pub mod import {
    pub use super::super::import::*;
    pub use crate::{app::data::query::export::*};
    pub use diesel::prelude::*;
}

pub mod export {
    pub use super::scan::*;
    pub use super::state::*;
    pub use super::diff::*;
}
pub use export::*;

pub mod api {

    use super::import::*;
    use super::export::*;
    use crate::{
        model::{export::*, file, tag, file_tag},
        app::{attr::{File as Attributes}, data::query::{self, Ids}},
        util::file::UnixFileType,
    };

    fn query_scanned_file_ids(scan: &mut Scan, c: &db::Connection) -> Res<Vec<Fid>> {
        let mut scanned: Boolean =
            box files::path.eq_any(scan.take_files());
        for d in scan.take_directories() {
            let mut l = d.clone();
            l.push(std::path::MAIN_SEPARATOR);
            l.push('%');
            scanned = box scanned.or(files::path.eq(d).or(files::path.like(l)));
        }
        Ok(files::table.select(files::id).filter(scanned).get_results(c.get())?)
    }

    fn scan_database_and_filesystem(paths: &Vec<&str>, c: &db::Connection) -> Res<(Vec<Attributes>, query::Columns, query::ManyToMany)> {
        let mut scan = Scan::scan(paths);
        profile!("dbms", { c.get().transaction::<_, Error, _>(|| {
            let raw: Vec<Ids> =
                file_tags::table
                    .select(file_tag::IDS)
                    .filter(file_tags::file_id.eq_any(query_scanned_file_ids(&mut scan, c)?))
                    .get_results(c.get())?;
            let (columns, many_to_many) = query::api::query_associated(raw, c)?;
            Ok((scan.take_attributes(), columns, many_to_many))
        }) })
    }

    /// Since not all tags appear on the scanned path
    /// we take special care not to insert duplicate tags
    /// or delete others unneccessarily.
    fn deduplicate_tags<'u>(diff: &'u Diff, c: &db::Connection) -> Res<(DiffedStrIds<'u>, DiffedStrIds<'u>, HashMap<String, Tid>)> {
        let (del, ins) = diff.tag_diff();
        let ddup =
            tags::table
                .select((tags::name, tags::id))
                .filter(tags::name.eq_any(ins.clone().chain(del.clone()).map(|e| *e)))
                .get_results(c.get())?.into_iter()
                .collect();
        Ok((del, ins, ddup))
    }

    fn get_file_type(path: &str) -> UnixFileType {
        use std::fs::File;
        UnixFileType::from_std(
            &File::open(path)
                .expect("FIXME: improve error handling")
                .metadata()
                .expect("FIXME: improve error handling")
                .file_type()
        )
    }

    /// Generate updates and deletes for files using
    /// the diff of filesystem and database
    fn process_file_diff<'u>(diff: &'u Diff<'u>, ins: &mut Ins<'u>, del: &mut Del<'u>) {
        let (fdel, fins) = diff.file_diff();
        for f in fins { ins.files.push(file::Insert { path: f, kind: get_file_type(f).to_i64() }); }
        for f in fdel { del.files.push(f); }
    }

    /// Generate updates and deletes for tags using
    /// the diff of filesystem and database
    fn process_tag_diff<'u>(diff: &'u Diff<'u>, ins: &mut Ins<'u>, del: &mut Del<'u>, c: &db::Connection) -> Res<HashMap<String, Tid>> {
        let (tdel, tins, ddup) = deduplicate_tags(diff, c)?;
        for t in tins { if !ddup.contains_key(*t) { ins.tags.push(tag::Insert { name: t }); }}
        for t in tdel { if !ddup.contains_key(*t) { del.tags.push(t); }}
        Ok(ddup)
    }

    /// Generate updates and deletes for filetags using
    /// the diff of filesystem and database
    fn process_filetag_diff(diff: &Diff, ins: &mut Ins, del: &mut Del, maps: &Maps) {
        let get = |f, t| { (maps.fids().by_alt(f).unwrap(), maps.tids().by_alt(t).unwrap()) };
        let (pdel, pins) = diff.filetag_diff();
        for (file, tag) in pins {
            let (f, t) = get(*file, *tag);
            ins.filetags.push(FileTag { file_id: f, tag_id: t });
        }
        for (file, tag) in pdel {
            let (f, t) = get(*file, *tag);
            del.filetags.push((f, t));
        }
    }

    /// Insert, map and delete files
    fn process_files<'u>(ins: &Ins, del: &Del, c: &db::Connection) -> Res<Vec<File>> {
        File::delete_paths(&del.files, c)?;
        File::insert_all(&ins.files, c)
    }

    /// Insert, map and delete tags
    fn process_tags<'u>(ins: &Ins, del: &Del, c: &db::Connection) -> Res<Vec<Tag>> {
        Tag::delete_names(&del.tags, c)?;
        Tag::insert_all(&ins.tags, c)
    }

    /// Insert and delete filetags
    fn process_filetags<'u>(ins: &Ins, del: &Del, c: &db::Connection) -> Res<()> {
        FileTag::insert_all(&ins.filetags, c)?;
        FileTag::delete_ids(&del.filetags, c)?;
        Ok(())
    }

    /// The main update routine:
    ///   1. Scan for data in the database and filesystem and map the IDs
    ///   2. Generate a 'diff' of found items
    ///   3. Insert missing files and tags and map their IDs
    ///   4. Insert missing filetags (now we know the IDs)
    ///   5. Forget any items that exist in the db and not in the fs
    pub fn run(paths: &Vec<&str>, c: &db::Connection) -> Res<()> {
        profile!("transaction", { c.get().transaction::<_, Error, _>(|| {
            let (attributes, columns, many_to_many) = profile!("queries", { scan_database_and_filesystem(paths, c)? });
            let mut maps = profile!("maps", { Maps::from_cols_and_mtom(&columns, many_to_many) });
            let diff = profile!("diff", { Diff::from_cols_and_attr(&columns, &attributes, &maps)? });
            let mut ins = Ins::new();
            let mut del = Del::new();
            let fins; let tins; let tdup;
            profile!("files", {
                profile!("diff", { process_file_diff(&diff, &mut ins, &mut del) });
                profile!("sql", {
                    fins = process_files(&ins, &del, c)?;
                    for f in fins.iter() { maps.add_file(f.id, &f.path); }
                })
            });
            profile!("tags", {
                tdup = profile!("diff", { process_tag_diff(&diff, &mut ins, &mut del, c)? });
                profile!("sql", {
                    tins = process_tags(&ins, &del, c)?;
                    for t in tdup.iter() { maps.add_tag(*t.1, t.0) }
                    for t in tins.iter() { maps.add_tag(t.id, &t.name); }
                });
            });
            profile!("filetags", {
                profile!("diff", { process_filetag_diff(&diff, &mut ins, &mut del, &maps) });
                profile!("sql", { process_filetags(&ins, &del, c)? });
            });
            info!("INSERT: {} File(s)", ins.files.len());
            info!("DELETE: {} File(s)", del.files.len());
            info!("INSERT: {} Tag(s)", ins.tags.len());
            info!("DELETE: {} Tag(s)", del.tags.len());
            info!("INSERT: {} FileTag(s)", ins.filetags.len());
            info!("DELETE: {} FileTag(s)", del.filetags.len());
            Ok(())
        })})
    }
}
