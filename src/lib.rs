#![feature(test, type_alias_enum_variants, rustc_private)]
#![allow(stable_features)]
#[macro_use] extern crate diesel;
#[macro_use] extern crate failure;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate serde;
extern crate clap;
extern crate internship;
extern crate md5;
extern crate owning_ref;
extern crate serde_yaml;
extern crate serde_json;
extern crate shell_escape;
extern crate test;
extern crate walkdir;
extern crate xattr;

#[macro_use] pub mod util;
pub mod app;
pub mod db;
pub mod dsl;
pub mod expression;
pub mod model;

pub mod import {
    pub use super::util::{self, error::*, profiler};
    pub use std::collections::{HashSet, HashMap};
    pub use std::path::{Path, PathBuf};
}

pub mod defaults {
    pub const CONFIG_HOME: &str = env!("XDG_CONFIG_HOME");
    pub const DATA_HOME: &str = env!("XDG_DATA_HOME");
    pub const HOME_NAME: &str = "tag";
    pub const DATABASE_PATH: &str = "db.sqlite";
    pub const CONFIG_NAME: &str = "config.yaml";

    pub fn config_home() -> String { format!("{}/{}", CONFIG_HOME, HOME_NAME) }
    pub fn config_path(path: &str) -> String { format!("{}/{}", config_home(), path) }
    pub fn data_home() -> String { format!("{}/{}", DATA_HOME, HOME_NAME) }
    pub fn data_path(path: &str) -> String { format!("{}/{}", data_home(), path) }

    pub const TEST_HOME: &str = "test";
    pub fn test_home(prefix: &str) -> String { format!("{}/{}", TEST_HOME, prefix) }
    pub fn test_path(prefix: &str, path: &str) -> String { format!("{}/{}", test_home(prefix), path) }
}
