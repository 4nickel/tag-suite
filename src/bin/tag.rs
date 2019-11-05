#![feature(test)]
#[macro_use] extern crate log;
extern crate env_logger;
extern crate clap;
extern crate tag_suite;

pub use tag_suite::{import::*, app::attr::{api, Tag, File}, util::arg::{Options}};
pub use clap::{App, ArgMatches, Arg, SubCommand};

#[derive(Debug)]
enum Command<'a> {
    Add(&'a str),
    Del(&'a str),
    Merge(&'a str, &'a str),
    Purge,
    Get,
    Nop,
}

struct Cli {}

impl Cli {

    pub fn add(file: &Path, tag: &str) -> Res<()> {
        let mut f = File::open(file.into())?;
        f.add(tag)?; f.save()?;
        Ok(())
    }

    pub fn del(file: &Path, tag: &str) -> Res<()> {
        let mut f = File::open(file.into())?;
        f.del(tag); f.save()?;
        Ok(())
    }

    pub fn get(file: &Path) -> Res<()> {
        let f = File::open(file.into())?;
        use std::io::{self, Write};
        match write!(io::stdout(), "{}\n", f.format()) {
            Err(e) => {
                Err(e.into())
            },
            _ => Ok(())
        }
    }

    pub fn purge(file: &Path) -> Res<()> {
        let mut f = File::open(file.into())?;
        f.purge(); f.save()?;
        Ok(())
    }

    pub fn merge(file: &Path, src: &str, dst: &str) -> Res<()> {
        let mut f = File::open(file.into())?;
        f.merge(src, dst)?; f.save()?;
        Ok(())
    }
}

fn main() -> Res<()> {

    env_logger::init();

    trace!("parsing CLI options");
    let args = App::new("xtg")
        .version("0.1")
        .about("tagging tools")
        .author("Felix V.")

        .subcommand(SubCommand::with_name("add")
            .about("Add a tag to files")
            .arg(Arg::with_name("TAG")
                .help("The tag to add")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("FILE")
                .help("The file(s) to tag")
                .required(true)
                .takes_value(true)
                .multiple(true)))

        .subcommand(SubCommand::with_name("get")
            .about("Get tag data from files")
            .arg(Arg::with_name("FILE")
                .help("The file(s) to query")
                .required(true)
                .takes_value(true)
                .multiple(true)))

        .subcommand(SubCommand::with_name("del")
            .about("Remove a tag from files")
            .arg(Arg::with_name("TAG")
                .help("The tag to add")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("FILE")
                .help("The file(s) to untag")
                .required(true)
                .takes_value(true)
                .multiple(true)))

        .subcommand(SubCommand::with_name("purge")
            .about("Purge all tags from the given files")
            .arg(Arg::with_name("FILE")
                .help("The file(s) to query")
                .required(true)
                .takes_value(true)
                .multiple(true)))

        .subcommand(SubCommand::with_name("merge")
            .about("Merge the the source tag into the destination - equivalent to renaming a tag")
            .arg(Arg::with_name("SRC")
                .help("The tag to merge")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("DST")
                .help("The tag to merge into")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("FILE")
                .help("The file(s) to operate on")
                .required(true)
                .takes_value(true)
                .multiple(true)))

        .get_matches();

    let (files, command) = {
        if let Some(options) = args.subcommand_matches("add") {
            (Options::_vec(options, "FILE"), Command::Add(Options::_get(options, "TAG")))
        } else if let Some(options) = args.subcommand_matches("del") {
            (Options::_vec(options, "FILE"), Command::Del(Options::_get(options, "TAG")))
        } else if let Some(options) = args.subcommand_matches("get") {
            (Options::_vec(options, "FILE"), Command::Get)
        } else if let Some(options) = args.subcommand_matches("purge") {
            (Options::_vec(options, "FILE"), Command::Purge)
        } else if let Some(options) = args.subcommand_matches("merge") {
            (Options::_vec(options, "FILE"), Command::Merge(Options::_get(options, "SRC"), Options::_get(options, "DST")))
        } else { (Vec::new(), Command::Nop) }
    };
    trace!("command: {:?}", command);

    let mut errors = Vec::new();
    let mut err = |f: &Path, r| {
        match r {
            Err(e) => errors.push((f.to_owned(), e)),
            _ => {},
        }
    };

    trace!("running command..");
    let files = files.iter().map(|f| Path::new(f));
    match command {
        Command::Del(tag) => {
            trace!("deleting '{}' from {} files", tag, files.len());
            for file in files { err(&file, Cli::del(file, tag)); }
        }
        Command::Add(tag) => {
            trace!("adding '{}' to {} files", tag, files.len());
            for file in files { err(&file, Cli::add(file, tag)); }
        }
        Command::Get => {
            trace!("querying {} files", files.len());
            for file in files { err(&file, Cli::get(file)); }
        }
        Command::Merge(src, dst) => {
            trace!("merging '{}' into '{}' in {} files", src, dst, files.len());
            for file in files { err(&file, Cli::merge(file, src, dst)); }
        }
        Command::Purge => {
            trace!("purging {} files", files.len());
            for file in files { err(&file, Cli::purge(file)); }
        }
        Command::Nop => { }
    }

    if errors.len() > 0 {
        error!("{} error(s) occurred:", errors.len());
    }

    for (file, error) in errors {
        error!("{}: {}", error, file.to_string_lossy());
    }

    Ok(())
}

#[cfg(test)]
mod suite {
    // use super::*;
    // use test::Bencher;
    // #[bench]
    // fn bench_10000_adds_and_deletes() {
    // }
}
