use crate::shell::{self, Shell};
use itertools::Itertools;
use std::{fs::File, io, io::Read};

static DEFAULT_LENGTH: usize = 1000;

trait HistoryParser {
    fn parse(&self, contents: &str, length: usize) -> Vec<String>;
}

struct BashParser;
impl HistoryParser for BashParser {
    fn parse(&self, _contents: &str, _length: usize) -> Vec<String> {
        unimplemented!()
    }
}

struct FishParser;
impl HistoryParser for FishParser {
    fn parse(&self, _contents: &str, _length: usize) -> Vec<String> {
        unimplemented!()
    }
}

struct ZshParser;
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
        let contents = self.file_contents(file)?;

        Ok(strategy.parse(&contents, self.length))
    }

    fn file_contents(&self, mut file: File) -> Result<String, io::Error> {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
}
