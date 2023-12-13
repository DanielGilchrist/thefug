use crate::shell::{self, Shell};

use itertools::Itertools;
use regex::Regex;

use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

static DEFAULT_LENGTH: usize = 1000;

trait Parser {
    fn parse(&self, buf_reader: BufReader<File>, length: usize) -> Vec<String>;
}

struct BashParser;
impl Parser for BashParser {
    // echo "hello"
    // echo "ok" && echo "hmm" (this is a multi-line command)
    // cat ~/.zsh_history
    // cargo build
    // exit
    fn parse(&self, buf_reader: BufReader<File>, length: usize) -> Vec<String> {
        buf_reader
            .lines()
            .map_while(Result::ok)
            .collect_vec()
            .into_iter()
            .rev()
            .take(length)
            .collect_vec()
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

impl Parser for FishParser {
    // - cmd: echo alpha
    //   when: 1339717374
    // - cmd: function foo\necho bar\nend
    //   when: 1339717377
    // - cmd: echo this has\\\nbackslashes
    //   when: 1339717385
    fn parse(&self, buf_reader: BufReader<File>, length: usize) -> Vec<String> {
        let fish_regex = Regex::new(r"\s*cmd:\s*(.+)$").unwrap();

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
impl Parser for ZshParser {
    // : 1679749063:0;cargo fmt
    // : 1679750298:0;echo "ok" \\
    //   && echo "hmm"
    // : 1679750300:0;cat ~/.zsh_history
    fn parse(&self, buf_reader: BufReader<File>, length: usize) -> Vec<String> {
        let mut commands = vec![];
        let mut current_command = String::new();

        for line in buf_reader.lines() {
            match line {
                Ok(line) => {
                    if line.starts_with(':') {
                        // Push the previous commmand
                        if !current_command.is_empty() {
                            commands.push(current_command.trim().to_string());
                        }

                        let command = line.split(';').last().unwrap().trim().to_string();

                        current_command.clear();
                        current_command.push_str(&command);
                    } else {
                        // Append to current command
                        current_command.push_str(&line);
                    }
                }
                Err(_) => {
                    // Clear current command and skip line
                    current_command.clear();
                }
            }
        }

        // Push the last command
        if !current_command.is_empty() {
            commands.push(current_command.trim().to_string());
        }

        commands.reverse();
        commands.truncate(length);

        commands
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

    fn _parse<T: Parser>(&self, parser: T) -> Result<Vec<String>, io::Error> {
        let location = self
            .shell
            .history_location()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "History location not found"))?;

        let file = File::open(location)?;
        let buf_reader = BufReader::new(file);

        Ok(parser.parse(buf_reader, self.length))
    }
}
