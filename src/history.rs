use std::{fs::File, io, io::Read};

use itertools::Itertools;

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
    history_path: String,
}

impl History {
    pub fn new(history_path: String) -> History {
        History {
            length: DEFAULT_LENGTH,
            history_path,
        }
    }

    pub fn parse<T: HistoryParser>(&self, strategy: T) -> Result<Vec<String>, io::Error> {
        let mut file = File::open(&self.history_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let string = String::from_utf8_lossy(&buffer).to_string();

        Ok(strategy.parse(&string, self.length))
    }
}
