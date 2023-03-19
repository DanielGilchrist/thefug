use std::{fs::{self, File}, io::{BufReader, BufRead, Read}};

static DEFAULT_LENGTH: i32 = 100;

pub struct History {
  length: i32
}

impl History {
  pub fn new(length: i32) -> History {
    History {
      length: DEFAULT_LENGTH
    }
  }

  pub fn get(&self) {
    match self.history_contents() {
      Some(history_contents) => {
        let history_lines: Vec<&str> = history_contents.split('\n').into_iter().filter_map(|line| {
          let line_parts: Vec<&str> = line.split(';').into_iter().collect();
          line_parts.get(1).map(|part| *part)
        }).collect::<Vec<_>>();

        println!("{:?}", history_lines);
      },

      None => println!("Error finding history!")
    };
  }

  fn history_contents(&self) -> Option<String> {
    let home = std::env::var("HOME").unwrap();
    let history_file = format!("{home}/.zsh_history");
    println!("{:?}", history_file);

    // fs::read_to_string(history_file).unwrap()

    let mut file = File::open(history_file).ok()?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let string = String::from_utf8_lossy(&buffer).to_string();
    Some(string)
  }
}

impl Default for History {
  fn default() -> History {
    History {
      length: DEFAULT_LENGTH
    }
  }
}
