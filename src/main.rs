mod command_matcher;
mod history;

use crate::command_matcher::CommandMatcher;
use crate::history::History;

fn main() {
    match History::new().get() {
        Ok(history) => {
            let dummy_command = "cargo bun";
            let matcher = CommandMatcher::new(history);

            match matcher.find_match(dummy_command) {
                Some(mut suggestions) => {
                    suggestions.sort_by(|suggestion1, suggestion2| {
                        suggestion1
                            .similarity
                            .partial_cmp(&suggestion2.similarity)
                            .unwrap()
                    });
                    suggestions.reverse();

                    let suggested_commands = suggestions
                        .into_iter()
                        .map(|suggestion| suggestion.command)
                        .collect::<Vec<String>>();

                    println!("Suggesions:\n{:?}", suggested_commands);
                }

                None => {
                    println!("No suggestions dB^(");
                }
            }
        }

        Err(error) => {
            eprintln!("{:?}", error);
        }
    }
}
