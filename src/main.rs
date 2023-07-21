mod command_matcher;
mod history;
mod init;
mod selector;
mod shell;

use crate::{
    command_matcher::CommandMatcher, history::History, init::Init, selector::Selector, shell::Shell,
};

use clap::Parser;

static MAX_SUGGESTIONS: usize = 5;
static NO_SUGGESTION_NEEDED: &str = "No fugs given.";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Options {
    #[clap(long)]
    init: bool,

    #[clap(long)]
    initdev: bool,
}

struct CommandWithHistory {
    command: String,
    history: Vec<String>,
}

impl CommandWithHistory {
    pub fn new(command: String, history: Vec<String>) -> CommandWithHistory {
        CommandWithHistory { command, history }
    }

    pub fn from(mut history: Vec<String>) -> Option<CommandWithHistory> {
        if history.len() < 2 {
            return None;
        }

        // Drop command that executed this program
        history.swap_remove(0);
        let command = history.swap_remove(1);

        Some(CommandWithHistory::new(command, history))
    }
}

fn main() {
    let shell = Shell::default();
    let options = Options::parse();

    if options.init {
        match Init::new(shell).init() {
            Ok(_) => (),
            Err(error) => eprintln!("{:?}", error),
        }

        return;
    }

    if options.initdev {
        match Init::new(shell).init_dev() {
            Ok(_) => (),
            Err(error) => eprintln!("{:?}", error),
        }

        return;
    }

    let history = match History::new(shell).parse() {
        Ok(history) => history,
        Err(error) => {
            eprintln!("{:?}", error);
            return;
        }
    };

    let command_with_history = match CommandWithHistory::from(history) {
        Some(command_with_history) => command_with_history,
        None => return no_fugs_given(),
    };

    let suggestions = {
        match CommandMatcher::new(command_with_history.history)
            .find_match(&command_with_history.command)
        {
            Some(mut suggestions) => {
                suggestions.sort_by(|suggestion1, suggestion2| {
                    suggestion1
                        .similarity
                        .partial_cmp(&suggestion2.similarity)
                        .unwrap()
                });

                suggestions.reverse();

                suggestions
            }
            None => return no_fugs_given(),
        }
    };

    let suggested_commands = suggestions
        .into_iter()
        .map(|suggestion| suggestion.command)
        .take(MAX_SUGGESTIONS)
        .collect::<Vec<String>>();

    let Ok(selected_command) = Selector::new(command_with_history.command, suggested_commands).show() else {
      return no_fugs_given();
    };

    println!("{selected_command}");
}

fn no_fugs_given() {
    println!("{NO_SUGGESTION_NEEDED}");
}
