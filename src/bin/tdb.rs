#![feature(box_syntax)]
#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate xtag;
extern crate env_logger;
extern crate clap;
extern crate owning_ref;

use clap::{App, Arg, SubCommand, ArgMatches};
use xtag::{import::*, db::export::*, util::{arg::Options}};
use xtag::app::data::{DatabaseLayer, query::{self, collect}};
use xtag::app::meta::{Configuration, config};
use xtag::util::profiler;
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
pub enum TagCommand {
    List,
    Clean,
    Statistics,
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
    Tag(TagCommand),
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
        use xtag::app::meta::config::{Config as AppConfig};
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

    /// The 'forget' command
    pub fn update(&mut self, paths: &Vec<&str>) -> Res<()> {
        self.dapi.update(paths)?;
        Ok(())
    }

    pub fn tag_statistics(&self) -> Res<()> {
        // let mut stats = self.dapi.query_tag_statistics()?;
        // for ((a, b), c) in stats {
        //     println!("{}|{} -> {}", a, b, c);
        // }
        Ok(())
    }

    pub fn tag_list(&self) -> Res<()> {
        let mut tags = self.dapi.query_all_tags()?;
        tags.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let output = tags.into_iter()
            .fold(String::new(), |mut s, (_, name)| {
                if !name.starts_with("tdb::") {
                    s.push_str(&name);
                    s.push('\n');
                }
                s
            });
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
        self.dapi.query_map(query::Pipeline::from_pipeline(pipeline)?, action, commit)?;
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
        self.dapi.enforce(&self.conf.conventions, commit)?;
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
                } else if let Some(_options) = options.subcommand_matches("statistics") {
                    Command::Tag(TagCommand::Statistics)
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
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline).expand(cli.config().expansions())?;
                        cli.query(pipe, false)?;
                    }
                    QueryCommand::Map(pipeline, map, commit) => {
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline).expand(cli.config().expansions())?;
                        cli.query_map(pipe, map, commit)?;
                    }
                    QueryCommand::Count(pipeline) => {
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline).expand(cli.config().expansions())?;
                        cli.query(pipe, true)?;
                    }
                    QueryCommand::Serialize(pipeline, format) => {
                        let pipe = config::PipelineBuf::from_pipeline(&pipeline).expand(cli.config().expansions())?;
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
            Command::Tag(TagCommand::Statistics) => {
                cli.tag_statistics()?;
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
            .author("Felix V.")

            .arg(Arg::with_name("database")
                .short("d")
                .long("database")
                .help("Use the given sqlite database FILE")
                .value_name("FILE")
                .takes_value(true))

            .subcommand(SubCommand::with_name("config")
                .about("Configuration subcommand"))

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
                    .about("Generate tag statistics")))

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

            .subcommand(SubCommand::with_name("forget")
                .about("Queries the database and forgets the results")
                .arg(Arg::with_name("filter")
                    .short("f")
                    .long("filter")
                    .help("Filter the results using EXPR")
                    .value_name("EXPR")
                    .takes_value(true))
                .arg(Arg::with_name("pipe")
                    .short("p")
                    .long("pipe")
                    .help("Pipe the results through SH")
                    .value_name("SH")
                    .takes_value(true))
                .arg(Arg::with_name("clean")
                    .short("c")
                    .long("clean")
                    .help("Enforce a clean afterwards"))
                .arg(Arg::with_name("QUERY")
                    .help("The QUERY to enforce")
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
                    .help("Pipe the results through SH")
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
                        .help("Call a subcommand on every file in a query")
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
                            .help("The TAG to be merged")
                            .required(true)
                            .takes_value(true))
                        .arg(Arg::with_name("DST")
                            .help("The TAG to merge into")
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
