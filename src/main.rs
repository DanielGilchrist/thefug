mod command_matcher;
mod history;
mod selector;

use std::{io, io::Write, process::Command};

use crate::{command_matcher::CommandMatcher, history::History, selector::Selector};

fn main() {
    let history = match History::new().get() {
        Ok(history) => history,
        Err(error) => {
            eprintln!("{:?}", error);
            return;
        }
    };

    let dummy_command = String::from("cargo biuld");
    let Some(mut suggestions) = CommandMatcher::new(history).find_match(&dummy_command) else {
        eprintln!("No suggestions found");
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
        .take(5)
        .collect::<Vec<String>>();

    let Ok(selected_command) = Selector::new(dummy_command, suggested_commands).show() else {
      eprintln!("No command selected");
      return
    };

    execute_command(&selected_command);
}

fn execute_command(command: &str) {
    let command_with_args = command.split_whitespace().collect::<Vec<_>>();

    println!("Running {command}...");
    let selection = Command::new(command_with_args[0])
        .arg(command_with_args[1])
        .output()
        .unwrap_or_else(|_| panic!("Failed to run {command}"));

    // TODO - Figure out how to write with colour
    io::stdout().write_all(&selection.stdout).unwrap();
    io::stderr().write_all(&selection.stderr).unwrap();
}
