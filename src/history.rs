use crate::shell::{self, Shell};

use itertools::Itertools;
use regex::Regex;

use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

static DEFAULT_LENGTH: usize = 1000;

trait HistoryParser {
    fn parse(&self, buf_reader: BufReader<File>, length: usize) -> Vec<String>;
}

struct BashParser;
impl HistoryParser for BashParser {
    fn parse(&self, _buf_reader: BufReader<File>, _length: usize) -> Vec<String> {
        unimplemented!()
    }
}

struct FishParser;
impl FishParser {
    fn parse_line(&self, line: Result<String, io::Error>, regex: &Regex) -> Option<String> {
        let line = line.ok()?;
        let captures = regex.captures(&line)?;

        captures.get(1).map(|m| m.as_str().to_owned())
    }
}

impl HistoryParser for FishParser {
    // - cmd: echo alpha
    //   when: 1339717374
    // - cmd: function foo\necho bar\nend
    //   when: 1339717377
    // - cmd: echo this has\\\nbackslashes
    //   when: 1339717385
    fn parse(&self, buf_reader: BufReader<File>, length: usize) -> Vec<String> {
        let fish_regex = Regex::new(r#"\s*cmd:\s*(.+)$"#).unwrap();

        buf_reader
            .lines()
            .filter_map(|line| self.parse_line(line, &fish_regex))
            .collect_vec()
            .into_iter()
            .unique()
            .rev()
            .take(length)
            .collect()
    }
}

struct ZshParser;
impl HistoryParser for ZshParser {
    fn parse(&self, buf_reader: BufReader<File>, length: usize) -> Vec<String> {
        // TODO: Refactor - this is a mess and doesn't handle edge cases
        buf_reader
            .lines()
            .filter_map(|line| line.ok())
            .unique()
            .collect_vec()
            .into_iter()
            .rev()
            .take(length)
            .filter_map(|line| {
                let line_parts = line.split(';').collect_vec();
                line_parts.get(1).map(|part| part.to_string())
            })
            .collect_vec()
    }
}

pub struct History {
    length: usize,
    shell: Shell,
}

impl History {
    pub fn new(shell: Shell) -> History {
        History {
            length: DEFAULT_LENGTH,
            shell,
        }
    }

    pub fn parse(&self) -> Result<Vec<String>, io::Error> {
        match self.shell.type_ {
            shell::Type::Bash => self._parse(BashParser),
            shell::Type::Fish => self._parse(FishParser),
            shell::Type::Zsh => self._parse(ZshParser),
            shell::Type::Unknown => unimplemented!(),
        }
    }

    fn _parse<T: HistoryParser>(&self, strategy: T) -> Result<Vec<String>, io::Error> {
        let location = self.shell.history_location().ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "History location not found",
        ))?;

        let file = File::open(location)?;
        let buf_reader = BufReader::new(file);

        Ok(strategy.parse(buf_reader, self.length))
    }
}
