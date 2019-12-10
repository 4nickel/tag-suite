#![feature(test, box_syntax)]
#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate tag_suite;
extern crate test;
extern crate env_logger;
extern crate clap;
extern crate owning_ref;

use clap::{App, Arg, SubCommand, ArgMatches};
use tag_suite::{import::*, db::export::*, util::{arg::Options}};
use tag_suite::app::data::{DatabaseLayer, query::{self, collect}};
use tag_suite::app::meta::{Configuration, config};
use tag_suite::util::profiler;
use std::io::{self, Write};
use collect::Stringify;

mod defaults {
    pub const CONFIG_HOME: &'static str = env!("XDG_CONFIG_HOME");
    pub const DATA_HOME: &'static str = env!("XDG_DATA_HOME");
    pub const HOME_NAME: &'static str = "tag";
    pub const DATABASE_PATH: &'static str = "db.sqlite";
    pub const CONFIG_NAME: &'static str = "config.yaml";


    //pub const CONFIG_AUTO_NAME: &'static str = "config.auto.yaml";

    pub fn config_home() -> String { format!("{}/{}", CONFIG_HOME, HOME_NAME) }
    pub fn config_path(path: &str) -> String { format!("{}/{}", config_home(), path) }
    pub fn data_home() -> String { format!("{}/{}", DATA_HOME, HOME_NAME) }
    pub fn data_path(path: &str) -> String { format!("{}/{}", data_home(), path) }

    #[cfg(test)] pub const TEST_HOME: &'static str = "test";
    #[cfg(test)] pub fn test_home(prefix: &str) -> String { format!("{}/{}", TEST_HOME, prefix) }
    #[cfg(test)] pub fn test_path(prefix: &str, path: &str) -> String { format!("{}/{}", test_home(prefix), path) }
}

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "argument error: '{}'", message)]
        ArgumentError { message: String, },
    }
}
use error::{Error as E};

/// TODO: Should have named fields here, tbh
#[derive(Debug, Clone)]
pub enum QueryCommand<'a> {
    Output(config::Pipeline<'a>),
    Map(config::Pipeline<'a>, config::CommandAction<'a>, bool),
    Count(config::Pipeline<'a>),
    Serialize(config::Pipeline<'a>, Option<&'a str>),
}

#[derive(Debug, Clone)]
pub enum TagCommand<'a> {
    List,
    Clean,
    Statistics(Option<&'a str>),
}

#[derive(Debug, Clone)]
pub enum ConventionCommand {
    Record,
    Enforce(bool),
}

/// A cli command
#[derive(Debug, Clone)]
pub enum Command<'a> {
    Update(Vec<&'a str>, bool),
    Query(QueryCommand<'a>),
    Convention(ConventionCommand),
    Tag(TagCommand<'a>),
    Nop,
}

/// Command-line config options
pub struct Config {
    database: String,
    config: String,
}

/// The cli is mainly just scaffolding to
/// wire-up argument parsing
pub struct Cli {
    pool: db::SqlitePool,
    dapi: DatabaseLayer,
    conf: Configuration,
}

/// Rusts stdlib has the annoying habit of
/// producing broken pipe errors when used
/// in shell pipelines. We're not too worried
/// about broken pipes, so just discard these
/// errors entirely.
fn hide_spurious_pipe_errors<T>(res: Result<T, std::io::Error>) -> Res<()> {
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::BrokenPipe => Ok(()),
                _ => Err(e.into())
            }
        }
    }
}

impl Cli {

    pub fn new(config: &Config) -> Res<Self> {
        use tag_suite::app::meta::config::{Config as AppConfig};
        let c = profile!("file", { AppConfig::read(&config.config)? });
        let conf = profile!("conf", { Configuration::configure(c)? });
        let pool = profile!("pool", { db::Connection::new_pool(&config.database, 2)? });
        let dapi = profile!("connect", { DatabaseLayer::new(db::Connection(pool.get().expect("database connection failure"))) });
        Ok(Self { pool, conf, dapi })
    }

    pub fn connect(&self) -> db::Connection {
        db::Connection(self.pool.get().expect("database connection failure"))
    }

    pub fn data(&self) -> &DatabaseLayer {
        &self.dapi
    }

    pub fn config(&self) -> &Configuration {
        &self.conf
    }

    /// The 'update' command
    pub fn update(&self, paths: &Vec<&str>) -> Res<()> {
        self.dapi.update(paths)?;
        Ok(())
    }

    fn format_tag_statistics(names: (&str, &str), count: usize, percents: (f64, f64)) -> String {
        format!("{:50} | {:>6} | {:>1.3} : {:>1.3}",
            format!("{} : {}", names.0, names.1),
            count,
            percents.0, percents.1)
    }

    pub fn tag_statistics(&self, tag: Option<&str>) -> Res<()> {
        let stats = self.dapi.query_tag_statistics()?;
        let names = self.dapi.query_all_tags_mapped()?;
        for (a, b, n) in stats.sorted_pairs_with_names(&names) {
            let oa = stats.occurrences(a.id);
            let ob = stats.occurrences(b.id);
            assert!(oa != 0 && ob != 0, "bug: occurrence count malfunction");

            let pa = n as f64 / oa as f64;
            let pb = n as f64 / ob as f64;
            let names = (a.name, b.name);
            let percents = (pa, pb);
            if let Some(tag) = tag {
                if a.name == tag {
                    let s = Self::format_tag_statistics(names, n, percents);
                    println!("{}", s);
                }
            } else {
                let s = Self::format_tag_statistics(names, n, percents);
                println!("{}", s);
            }
        }
        Ok(())
    }

    pub fn tag_list(&self) -> Res<()> {
        use tag_suite::app::data::tag;
        let tags = self.dapi.query_all_tags_sorted()?;
        let output = tag::collect_to_string(tags);
        profile!("output", { write!(io::stdout(), "{}", output) })?;
        Ok(())
    }

    /// The 'query' command
    pub fn query(&self, pipeline: config::PipelineBuf, count: bool) -> Res<()> {
        let pipe = query::Pipeline::from_pipeline(pipeline)?;
        let output = if count {
            self.dapi.query_collect(&pipe, collect::FileCount)??.stringify()
        } else {
            self.dapi.query_collect(&pipe, collect::FilePaths)??
        };
        let io = profile!("output", { write!(io::stdout(), "{}", output) });
        hide_spurious_pipe_errors(io)?;
        Ok(())
    }

    /// The 'serialize' command
    pub fn query_serialize(&self, pipeline: config::PipelineBuf, format: Option<&str>) -> Res<()> {
        let pipe = query::Pipeline::from_pipeline(pipeline)?;
        let output = match format {
            Some("yaml") => { self.dapi.query_collect(&pipe, collect::SerializeYaml)?? }
            Some("json") => { self.dapi.query_collect(&pipe, collect::SerializeJson)?? }
            Some("plain") => { self.dapi.query_collect(&pipe, collect::SerializePlain)?? }
            None => { self.dapi.query_collect(&pipe, collect::SerializePlain)?? }
            Some(e) => { return Err(E::ArgumentError { message: e.into() }.into()) }
        };
        let io = profile!("output\n", { write!(io::stdout(), "{}", output) });
        hide_spurious_pipe_errors(io)?;
        Ok(())
    }

    /// The 'map' subcommand
    pub fn query_map(&mut self, pipeline: config::PipelineBuf, action: config::CommandAction, commit: bool) -> Res<()> {
        let summary = self.dapi.query_map(query::Pipeline::from_pipeline(pipeline)?, action, commit)?;
        let s = summary.format();
        println!("{}", s);
        Ok(())
    }

    /// The 'clean' command
    pub fn clean(&mut self) -> Res<usize> {
        match self.dapi.clean() {
            Ok(cleaned) => { info!("cleaned: {} Tags", cleaned); Ok(cleaned) }
            e => e,
        }
    }

    /// The 'enforce' subcommand
    pub fn enforce(&mut self, commit: bool) -> Res<()> {
        let reports = self.dapi.enforce(&self.conf.conventions, commit)?;
        for report in reports {
            let r = report.format();
            println!("{}", r);
        }
        Ok(())
    }
}

fn cli(options: &ArgMatches) -> Res<()> {
    //{{{ Options
    let o = Options::new(options);
    let oo; let ooo; let oooo;
    let (config, command) = profile!("options", {
        let config = Config {
            database: o.opt("database").map(|s| s.to_string())
                .unwrap_or(defaults::data_path(defaults::DATABASE_PATH)),
            config: o.opt("config").map(|s| s.to_string())
                .unwrap_or(defaults::config_path(defaults::CONFIG_NAME)),
        };
        let command = {
            if let Some(options) = options.subcommand_matches("update") {
                oo = Options::new(options); Command::Update(oo.vec("PATH"), oo.flag("clean"))
            } else if let Some(options) = options.subcommand_matches("tag") {
                /* oo = Options::new(options); */
                if let Some(_options) = options.subcommand_matches("clean") {
                    Command::Tag(TagCommand::Clean)
                } else if let Some(options) = options.subcommand_matches("statistics") {
                    ooo = Options::new(options);
                    Command::Tag(TagCommand::Statistics(ooo.opt("TAG")))
                } else if let Some(_options) = options.subcommand_matches("list") {
                    Command::Tag(TagCommand::List)
                } else {
                    Command::Tag(TagCommand::List)
                }
            } else if let Some(options) = options.subcommand_matches("query") {
                oo = Options::new(options);
                if let Some(_options) = options.subcommand_matches("count") {
                    let pipeline = config::Pipeline::from_options(&oo);
                    Command::Query(QueryCommand::Count(pipeline))
                } else if let Some(options) = options.subcommand_matches("serialize") {
                    ooo = Options::new(options);
                    let pipeline = config::Pipeline::from_options(&oo);
                    Command::Query(QueryCommand::Serialize(pipeline, ooo.opt("FORMAT")))
                } else if let Some(options) = options.subcommand_matches("map") {
                    ooo = Options::new(options);
                    let map = if let Some(options) = options.subcommand_matches("add") {
                        oooo = Options::new(options); config::CommandAction::Add(oooo.vec("TAGS"))
                    } else if let Some(_options) = options.subcommand_matches("forget") {
                        /* oooo = Options::new(options); */ config::CommandAction::Forget
                    } else if let Some(options) = options.subcommand_matches("del") {
                        oooo = Options::new(options); config::CommandAction::Del(oooo.vec("TAGS"))
                    } else if let Some(options) = options.subcommand_matches("report") {
                        oooo = Options::new(options); config::CommandAction::Report(oooo.get("MSG"))
                    } else if let Some(options) = options.subcommand_matches("link") {
                        oooo = Options::new(options); config::CommandAction::Link(oooo.vec("DSTS"))
                    } else if let Some(options) = options.subcommand_matches("unlink") {
                        oooo = Options::new(options); config::CommandAction::Unlink(oooo.vec("DSTS"))
                    } else if let Some(options) = options.subcommand_matches("merge") {
                        oooo = Options::new(options); config::CommandAction::Merge(oooo.get("SRC"), oooo.get("DST"))
                    } else {
                        return Err(E::ArgumentError { message: "map requires a subcommand".into() }.into())
                    };
                    let pipeline = config::Pipeline::from_options(&oo);
                    Command::Query(QueryCommand::Map(pipeline, map, ooo.flag("commit")))
                } else {
                    let pipeline = config::Pipeline::from_options(&oo);
                    Command::Query(QueryCommand::Output(pipeline))
                }
            } else if let Some(options) = options.subcommand_matches("convention") {
                /* oo = Options::new(options); */
                if let Some(options) = options.subcommand_matches("enforce") {
                    ooo = Options::new(options); Command::Convention(ConventionCommand::Enforce(ooo.flag("commit")))
                } else if let Some(_options) = options.subcommand_matches("record") {
                    /* ooo = Options::new(options); */ Command::Convention(ConventionCommand::Record)
                } else {
                    return Err(E::ArgumentError { message: "convention requires a subcommand".into() }.into())
                }
            } else { Command::Nop }
        };
        info!("command: {:?}", command);
        info!("config: {}", &config.config);
        info!("database: {}", &config.database);
        (config, command)
    });
    //}}}
    let mut cli = profile!("configure", { Cli::new(&config) })?;
    //{{{ Command
    profile!("command", {
        match command {
            Command::Update(paths, clean) => {
                cli.update(&paths)?;
                if clean { cli.clean()?; }
            }
            Command::Query(q) => {
                match q {
                    QueryCommand::Output(pipeline) => {
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline)
                            .expand(cli.config().expansions())?;
                        cli.query(pipe, false)?;
                    }
                    QueryCommand::Map(pipeline, map, commit) => {
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline)
                            .expand(cli.config().expansions())?;
                        cli.query_map(pipe, map, commit)?;
                    }
                    QueryCommand::Count(pipeline) => {
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline)
                            .expand(cli.config().expansions())?;
                        cli.query(pipe, true)?;
                    }
                    QueryCommand::Serialize(pipeline, format) => {
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline)
                            .expand(cli.config().expansions())?;
                        cli.query_serialize(pipe, format)?;
                    }
                }
            }
            Command::Tag(TagCommand::Clean) => {
                cli.clean()?;
            }
            Command::Tag(TagCommand::List) => {
                cli.tag_list()?;
            }
            Command::Tag(TagCommand::Statistics(tag)) => {
                cli.tag_statistics(tag)?;
            }
            Command::Convention(ConventionCommand::Enforce(commit)) => {
                cli.enforce(commit)?;
            }
            Command::Convention(ConventionCommand::Record) => {
                //cli.enforce(commit)?;
            }
            Command::Nop => {}
        }
    });
    //}}}
    Ok(())
}

fn main() -> Res<()> {

    env_logger::init();
    let exit = profile!("main", {
        //{{{ Clap
        let options =
            profile!("clap", {
            App::new("xtg")
            .version("0.1")
            .about("tagging tools")
            .author("Felix Viernickel")

            .arg(Arg::with_name("database")
                .short("d")
                .long("database")
                .help("Use the given sqlite database FILE")
                .value_name("FILE")
                .takes_value(true))

            .subcommand(SubCommand::with_name("config")
                .about("Configuration subcommand [unimplemented]"))

            .subcommand(SubCommand::with_name("convention")
                .about("Convention subcommand")
                .subcommand(SubCommand::with_name("enforce")
                    .about("Enforce conventions")
                    .arg(Arg::with_name("commit")
                        .short("c")
                        .long("commit"))
                        .help("Commit the results"))
                .subcommand(SubCommand::with_name("record")
                    .about("Record a convention")
                    .arg(Arg::with_name("name")
                        .short("n")
                        .long("name"))
                        .help("Name of the convention")))

            .subcommand(SubCommand::with_name("tag")
                .about("Perform operations on tags")
                .subcommand(SubCommand::with_name("list")
                    .about("List tags"))
                .subcommand(SubCommand::with_name("clean")
                    .about("Clean tags"))
                .subcommand(SubCommand::with_name("statistics")
                    .about("Generate tag statistics")
                    .arg(Arg::with_name("TAG")
                        .help("Show only those statistics concerning TAG")
                        .takes_value(true))))

            .subcommand(SubCommand::with_name("update")
                .about("Updates the database")
                .arg(Arg::with_name("clean")
                    .short("c")
                    .long("clean")
                    .help("Clean tags after update")
                    .takes_value(false))
                .arg(Arg::with_name("PATH")
                    .help("The paths to update")
                    .takes_value(true)
                    .multiple(true)))

            .subcommand(SubCommand::with_name("query")
                .about("Queries the database and prints the results")
                .arg(Arg::with_name("filter")
                    .short("f")
                    .long("filter")
                    .help("Filter the results using EXPR")
                    .value_name("EXPR")
                    .takes_value(true))
                .arg(Arg::with_name("pipe")
                    .short("p")
                    .long("pipe")
                    .help("Pipe the results through a shell command")
                    .value_name("SH")
                    .takes_value(true))
                .arg(Arg::with_name("QUERY")
                    .help("The QUERY to enforce")
                    .takes_value(true))
                .subcommand(SubCommand::with_name("count"))
                .subcommand(SubCommand::with_name("serialize")
                    .arg(Arg::with_name("FORMAT")
                        .help("The format to output")
                        .required(false)
                        .takes_value(true)))
                .subcommand(SubCommand::with_name("map")
                    .about("Map an action over a query")
                    .arg(Arg::with_name("commit")
                        .short("c")
                        .long("commit"))
                        .help("Commit the results")
                    .arg(Arg::with_name("filter")
                        .short("f")
                        .long("filter")
                        .help("Filter the results using EXPR")
                        .value_name("EXPR")
                        .takes_value(true))
                    .arg(Arg::with_name("QUERY")
                        .help("The QUERY to enforce")
                        .takes_value(true))
                    .subcommand(SubCommand::with_name("forget"))
                    .subcommand(SubCommand::with_name("emit"))
                    .subcommand(SubCommand::with_name("add")
                        .arg(Arg::with_name("TAGS")
                            .help("The TAGs to add")
                            .required(true)
                            .takes_value(true)
                            .multiple(true)))
                    .subcommand(SubCommand::with_name("del")
                        .arg(Arg::with_name("TAGS")
                            .help("The TAGs to remove")
                            .required(true)
                            .takes_value(true)
                            .multiple(true)))
                    .subcommand(SubCommand::with_name("report")
                        .arg(Arg::with_name("MSG")
                            .help("Report a file")
                            .takes_value(true)))
                    .subcommand(SubCommand::with_name("link")
                        .arg(Arg::with_name("DSTS")
                            .help("Destination directory/directories")
                            .takes_value(true)
                            .multiple(true)))
                    .subcommand(SubCommand::with_name("unlink")
                        .arg(Arg::with_name("DSTS")
                            .help("Destination directory/directories")
                            .takes_value(true)
                            .multiple(true)))
                    .subcommand(SubCommand::with_name("merge")
                        .arg(Arg::with_name("SRC")
                            .help("The tag to be merged")
                            .required(true)
                            .takes_value(true))
                        .arg(Arg::with_name("DST")
                            .help("The tag to merge into")
                            .required(true)
                            .takes_value(true)))))
            .get_matches()
        });
        //}}}
        profile!("exec", { cli(&options) })
    });

    match exit {
        Ok(()) => { print_profiler_analysis(0, 0, &profiler::analysis()); }
        Err(e) => { error!("{:?}", e); }
    }

    Ok(())
}

//{{{ Profiler stuff. TODO: move this!

fn print_profiler_analysis(indent: usize, handle: profiler::Handle, analysis: &profiler::Analysis) {
    let (_, stats) = analysis.frame(handle);
    print_frames(indent, handle, analysis);
    for (child, _) in stats.tally() {
        print_profiler_analysis(indent + 1, *child, analysis);
    }
}

fn print_frames(mut indent: usize, handle: profiler::Handle, analysis: &profiler::Analysis) {
    let (frame, stats) = analysis.frame(handle);
    let (tss, tms, tus, tns) = nanoseconds_to_human_time(stats.total());
    let mut indented = String::new();
    while indent > 0 {
        indent -= 1usize;
        indented.push_str("  ");
    }
    let mut tpct = stats.total_percent(analysis) * 100.0f64;
    let mut cpct = stats.child_percent(analysis) * 100.0f64;
    if tpct >= 99.99f64 { tpct = 100.00f64; }
    if cpct >= 99.99f64 { cpct = 100.00f64; }
    trace!("{:30} {:>3} {:>3} {:>3} {:>3}    {:>6.2}% : {:>6.2}%",
        format!("{}+ {}", indented, frame.name()), tss, tms, tus, tns, tpct, cpct
    );
}

fn nanoseconds_to_human_time(ns: i64) -> (i64, i64, i64, i64) {
    let tt = ns;
    let ss = tt / 1_000_000_000i64;
    let tt = tt % 1_000_000_000i64;
    let ms = tt / 1_000_000i64;
    let tt = tt % 1_000_000i64;
    let us = tt / 1_000i64;
    let ns = tt % 1_000i64;
    (ss, ms, us, ns)
}

//}}}

#[cfg(test)]
pub mod suite {

    use super::*;

    #[bench]
    fn bench_update_1500_files(b: &mut test::Bencher) {
        let config = Config {
            config: defaults::test_path("tag", "config.yaml"),
            database: defaults::test_path("tag", "db.sqlite"),
        };
        b.iter(|| {
            let cli = Cli::new(&config).unwrap();
            cli.update(&vec![defaults::test_path("files", "1500").as_str()]).unwrap();
        });
    }

    #[bench]
    fn bench_query_all(b: &mut test::Bencher) {
        let config = Config {
            config: defaults::test_path("tag", "config.yaml"),
            database: defaults::test_path("tag", "db.sqlite"),
        };
        b.iter(|| {
            //let cli = Cli::new(&config).unwrap();
        });
    }
}
