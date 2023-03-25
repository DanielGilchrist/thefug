use crate::shell::{self, Shell};
use itertools::Itertools;
use std::{fs::File, io, io::Read};

static DEFAULT_LENGTH: usize = 1000;

pub trait HistoryParser {
    fn parse(&self, contents: &str, length: usize) -> Vec<String>;
}

pub struct ZshParser;
impl HistoryParser for ZshParser {
    fn parse(&self, contents: &str, length: usize) -> Vec<String> {
        contents
            .split('\n')
            .unique()
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
        let strategy = match self.shell.type_ {
            shell::Type::Zsh => ZshParser,
            shell::Type::Fish => unimplemented!(),
            shell::Type::Unknown => unimplemented!(),
        };

        self._parse(strategy)
    }

    fn _parse<T: HistoryParser>(&self, strategy: T) -> Result<Vec<String>, io::Error> {
        let location = self.shell.history_location().ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "History location not found",
        ))?;

        let mut file = File::open(location)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let string = String::from_utf8_lossy(&buffer).to_string();

        Ok(strategy.parse(&string, self.length))
    }
}
