mod command_matcher;
mod history;
mod selector;
mod shell;

use std::{io, io::Write, process::Command};

use crate::{command_matcher::CommandMatcher, history::History, selector::Selector, shell::Shell};

use itertools::Itertools;

static MAX_SUGGESTIONS: usize = 5;

fn main() {
    let shell = Shell::default();
    let mut history = match History::new(shell).parse() {
        Ok(history) => history,
        Err(error) => {
            eprintln!("{:?}", error);
            return;
        }
    };

    if history.len() < 2 {
        eprintln!("Not enough history to suggest commands");
        return;
    }

    // Drop command that executed this program
    history.swap_remove(0);
    let last_command = history.swap_remove(1);

    let Some(mut suggestions) = CommandMatcher::new(history).find_match(&last_command) else {
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
        .take(MAX_SUGGESTIONS)
        .collect::<Vec<String>>();

    let Ok(selected_command) = Selector::new(last_command, suggested_commands).show() else {
      eprintln!("No command selected");
      return;
    };

    execute_command(&selected_command);
}

fn execute_command(selected_command: &str) {
    let mut command_with_args = selected_command.split_whitespace().collect_vec();
    let command_head = command_with_args.remove(0);
    let mut command = Command::new(command_head);

    command_with_args.into_iter().for_each(|arg| {
        command.arg(arg);
    });

    println!("Running {selected_command}...");
    let selection = command
        .output()
        .unwrap_or_else(|_| panic!("Failed to run {selected_command}"));

    // TODO - Figure out how to write with colour
    io::stdout().write_all(&selection.stdout).unwrap();
    io::stderr().write_all(&selection.stderr).unwrap();
}
