mod command_matcher;
mod history;

use crate::command_matcher::CommandMatcher;
use crate::history::History;

fn main() {
    let history = match History::new().get() {
        Ok(history) => history,
        Err(error) => {
            eprintln!("{:?}", error);
            return;
        }
    };

    let dummy_command = "cargo bun";
    let Some(mut suggestions) = CommandMatcher::new(history).find_match(dummy_command) else {
        eprintln!("No suggestions dB^(");
        return;
    };

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
