use super::{import::*, maps::*};
use crate::{dsl::{filter as filter_dsl}, expression::Ast};
use std::{cell::RefCell, process::Command};

pub trait Filter {
    fn filter<I>(&self, iter: I) -> Vec<Fid>
    where
        I: Iterator<Item=Fid>;
}

pub struct DslFilter<'q> {
    maps: &'q Maps<'q>,
    ast: &'q Ast,
    context: RefCell<filter_dsl::Context>,
    dsl: filter_dsl::Dsl<'q>,
}

impl<'q> DslFilter<'q> {
    pub fn new(maps: &'q Maps<'q>, ast: &'q Ast) -> Self {
        Self {
            maps: maps,
            context: RefCell::new(filter_dsl::Context::new()),
            dsl: filter_dsl::Dsl::new(),
            ast: ast,
        }
    }

    fn filter_item(&self, file: FileView<'q>) -> bool {
        self.dsl.evaluate(self.ast, &self.context, &file)
            .expect("filter error")
    }
}

impl<'q> Filter for DslFilter<'q> {
    fn filter<I>(&self, iter: I) -> Vec<Fid>
    where
        I: Iterator<Item=Fid>
    {
        iter.filter(|i|
            self.filter_item(self.maps.file(*i).unwrap())
        ).collect()
    }
}

pub struct PipeFilter<'q> {
    maps: &'q Maps<'q>,
    command: &'q str,
}

impl<'q> PipeFilter<'q> {
    pub fn new(maps: &'q Maps<'q>, command: &'q str) -> Self {
        Self { maps, command }
    }
}

impl<'q> Filter for PipeFilter<'q> {
    fn filter<I>(&self, iter: I) -> Vec<Fid>
    where
        I: Iterator<Item=Fid>
    {
        use std::process::Stdio;

        let files = iter.fold(String::new(), |mut acc, fid| {
            acc.push_str(self.maps.file(fid).unwrap().path());
            acc.push('\n');
            acc
        });

        // TODO: improve error handling
        let mut process = Command::new("sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .arg("-c")
            .arg(self.command)
            .spawn()
            .expect("failed to spawn child");

        use std::io::Write;
        let stdin = process.stdin.as_mut()
            .expect("failed to open stdin");
        stdin.write_all(files.as_bytes())
            .expect("failed to write to stdin");
        let output = process.wait_with_output()
            .expect("failed to read stdout");

        String::from_utf8_lossy(&output.stdout)
            .split("\n")
            .fold(Vec::new(), |mut acc, path|{
                if path.len() > 0 {
                    acc.push(self.maps.fids().by_alt(path).expect(&format!("unknown file: '{}'", path)));
                }
                acc
            })
    }
}
