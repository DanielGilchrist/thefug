use crate::shell::{self, Shell};

use itertools::Itertools;
use regex::Regex;

use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

static DEFAULT_LENGTH: usize = 1000;

trait Parser {
    fn parse<R: BufRead>(&self, reader: R, length: usize) -> Vec<String>;
}

struct BashParser;
impl Parser for BashParser {
    // echo "hello"
    // echo "ok" && echo "hmm" (this is a multi-line command)
    // cat ~/.zsh_history
    // cargo build
    // exit
    fn parse<R: BufRead>(&self, reader: R, length: usize) -> Vec<String> {
        reader
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
    fn parse<R: BufRead>(&self, reader: R, length: usize) -> Vec<String> {
        // `^- cmd:` so we don't match a `cmd:` substring inside a command.
        let fish_regex = Regex::new(r"^- cmd:\s*(.+)$").unwrap();

        reader
            .lines()
            .filter_map(|line| self.parse_line(line, &fish_regex))
            .collect_vec()
            .into_iter()
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
    // : 1679750301:0;git log; git status   <- command can contain `;`
    fn parse<R: BufRead>(&self, reader: R, length: usize) -> Vec<String> {
        let mut commands = vec![];
        let mut current_command = String::new();

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line.starts_with(':') {
                        if !current_command.is_empty() {
                            commands.push(current_command.trim().to_string());
                        }

                        // Split on the first `;` only — semicolons inside
                        // the command itself must be preserved.
                        let command = line
                            .split_once(';')
                            .map(|(_, rest)| rest.trim().to_string())
                            .unwrap_or_default();

                        current_command.clear();
                        current_command.push_str(&command);
                    } else {
                        current_command.push_str(&line);
                    }
                }
                Err(_) => {
                    current_command.clear();
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    fn parse<P: Parser>(parser: P, input: &str, length: usize) -> Vec<String> {
        parser.parse(Cursor::new(input), length)
    }

    mod bash {
        use super::*;

        #[test]
        fn empty_input() {
            assert!(parse(BashParser, "", 1000).is_empty());
        }

        #[test]
        fn single_line() {
            assert_eq!(parse(BashParser, "git pull\n", 1000), vec!["git pull"]);
        }

        #[test]
        fn returns_most_recent_first() {
            let input = "git pull\ngit push\ncargo test\n";

            assert_eq!(
                parse(BashParser, input, 1000),
                vec!["cargo test", "git push", "git pull"]
            );
        }

        #[test]
        fn truncates_to_length() {
            let input = "a\nb\nc\nd\ne\n";

            assert_eq!(parse(BashParser, input, 2), vec!["e", "d"]);
        }

        #[test]
        fn handles_no_trailing_newline() {
            assert_eq!(parse(BashParser, "git pull", 1000), vec!["git pull"]);
        }
    }

    mod fish {
        use super::*;

        #[test]
        fn empty_input() {
            assert!(parse(FishParser, "", 1000).is_empty());
        }

        #[test]
        fn extracts_single_cmd_entry() {
            let input = "- cmd: git pull\n  when: 1234567890\n";

            assert_eq!(parse(FishParser, input, 1000), vec!["git pull"]);
        }

        #[test]
        fn skips_non_cmd_lines() {
            let input = "\
- cmd: git pull
  when: 1234567890
- cmd: cargo test
  when: 1234567891
  paths:
    - /foo
";
            assert_eq!(
                parse(FishParser, input, 1000),
                vec!["cargo test", "git pull"]
            );
        }

        #[test]
        fn preserves_command_containing_cmd_substring() {
            let input = "- cmd: echo cmd: foo\n  when: 1234567890\n";

            assert_eq!(parse(FishParser, input, 1000), vec!["echo cmd: foo"]);
        }

        #[test]
        fn preserves_fish_literal_backslash_n_escapes() {
            // Fish stores multi-line commands as literal `\n` (two chars).
            let input = r"- cmd: function foo\necho bar\nend
  when: 1234567890
";
            assert_eq!(
                parse(FishParser, input, 1000),
                vec![r"function foo\necho bar\nend"]
            );
        }

        #[test]
        fn truncates_to_length() {
            let input = "\
- cmd: a
  when: 1
- cmd: b
  when: 2
- cmd: c
  when: 3
";
            assert_eq!(parse(FishParser, input, 2), vec!["c", "b"]);
        }
    }

    mod zsh {
        use super::*;

        #[test]
        fn empty_input() {
            assert!(parse(ZshParser, "", 1000).is_empty());
        }

        #[test]
        fn single_entry() {
            let input = ": 1679749063:0;cargo fmt\n";

            assert_eq!(parse(ZshParser, input, 1000), vec!["cargo fmt"]);
        }

        #[test]
        fn returns_most_recent_first() {
            let input = "\
: 1:0;git pull
: 2:0;git push
: 3:0;cargo test
";
            assert_eq!(
                parse(ZshParser, input, 1000),
                vec!["cargo test", "git push", "git pull"]
            );
        }

        #[test]
        fn preserves_semicolons_inside_command() {
            let input = ": 1679749063:0;git log; git status\n";

            assert_eq!(
                parse(ZshParser, input, 1000),
                vec!["git log; git status"]
            );
        }

        #[test]
        fn handles_multi_line_continuation() {
            let input = "\
: 1:0;echo \"ok\" \\
  && echo \"hmm\"
: 2:0;cargo build
";
            let result = parse(ZshParser, input, 1000);

            assert_eq!(result[0], "cargo build");
            assert!(
                result[1].contains("echo \"ok\"") && result[1].contains("&& echo \"hmm\""),
                "expected continuation to be joined: {:?}",
                result[1]
            );
        }

        #[test]
        fn truncates_to_length() {
            let input = "\
: 1:0;a
: 2:0;b
: 3:0;c
: 4:0;d
";
            assert_eq!(parse(ZshParser, input, 2), vec!["d", "c"]);
        }

        #[test]
        fn handles_no_trailing_newline() {
            let input = ": 1679749063:0;cargo fmt";

            assert_eq!(parse(ZshParser, input, 1000), vec!["cargo fmt"]);
        }
    }
}
