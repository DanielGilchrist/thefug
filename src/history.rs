use std::{fs::File, io, io::Read};

static DEFAULT_LENGTH: usize = 1000;

pub struct History {
    length: usize,
}

impl History {
    pub fn new() -> History {
        History {
            length: DEFAULT_LENGTH,
        }
    }

    pub fn get(&self) -> Result<Vec<String>, io::Error> {
        // TODO - Handle this error gracefully
        let home = std::env::var("HOME").unwrap();
        let file_name = format!("{home}/.zsh_history");

        self.history_contents(&file_name)
    }

    fn history_contents(&self, file_name: &str) -> Result<Vec<String>, io::Error> {
        let mut file = File::open(file_name)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let string = String::from_utf8_lossy(&buffer).to_string();
        let parsed_history = self.parse_history_contents(&string);

        Ok(parsed_history)
    }

    fn parse_history_contents(&self, contents: &str) -> Vec<String> {
        contents
            .split('\n')
            .rev()
            .take(self.length)
            .filter_map(|line| {
                let line_parts = line.split(';').collect::<Vec<_>>();
                line_parts.get(1).map(|part| part.to_string())
            })
            .collect::<Vec<_>>()
    }
}
