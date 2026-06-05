mod attempt;
mod history;
mod init;
mod parsed_command;
mod path_scan;
mod pipeline;
mod selector;
mod shell;
mod subcommand_scan;
mod suggestion;

use crate::{
    attempt::Attempt,
    history::History,
    init::Init,
    pipeline::{Pass, Pipeline},
    selector::Selector,
    shell::Shell,
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

fn main() {
    let shell = Shell::default();
    let options = Options::parse();

    if options.init {
        if let Err(error) = Init::new(shell).init() {
            eprintln!("{error}");
        }
        return;
    }

    if options.initdev {
        match Init::new(shell).init_dev() {
            Ok(()) => println!("Successfully initialized dev environment!"),
            Err(error) => eprintln!("{error}"),
        }
        return;
    }

    let history = match History::new(shell).parse() {
        Ok(history) => history,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let Some(attempt) = Attempt::from_shell_history(history) else {
        return no_fugs_given();
    };

    let pipeline = Pipeline::new(MAX_SUGGESTIONS)
        .add_pass(Pass::Subcommand)
        .add_pass(Pass::Path);
    let suggestions = pipeline.run(&attempt);

    if suggestions.is_empty() {
        return no_fugs_given();
    }

    let suggested_commands = suggestions
        .into_iter()
        .map(|suggestion| suggestion.command)
        .collect::<Vec<String>>();

    let Ok(selected_command) = Selector::new(attempt.failed_command, suggested_commands).show()
    else {
        return no_fugs_given();
    };

    println!("{selected_command}");
}

fn no_fugs_given() {
    println!("{NO_SUGGESTION_NEEDED}");
}
