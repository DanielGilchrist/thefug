use thefug::{
    attempt::Attempt,
    completions,
    history::History,
    init::Init,
    pipeline::{Pass, Pipeline},
    selector::Selector,
    shell::Shell,
    suggestion::Suggestion,
};

use clap::{Parser, Subcommand};

use std::path::PathBuf;

static MAX_SUGGESTIONS: usize = 5;
static NO_SUGGESTION_NEEDED: &str = "No fugs given.";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Options {
    #[command(subcommand)]
    mode: Option<Mode>,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Print shell integration to stdout (use with `eval "$(thefug init)"`)
    Init,
    /// Run against explicit inputs instead of shell history
    Simulate {
        /// The failed command to suggest corrections for
        command: String,

        /// Newline-delimited history file, most-recent first
        #[arg(long)]
        history: Option<PathBuf>,

        /// Print suggestions to stdout instead of opening the selector
        #[arg(long)]
        print: bool,
    },
}

fn main() {
    let shell = Shell::default();
    let options = Options::parse();

    match options.mode {
        Some(Mode::Init) => run_init(shell),
        Some(Mode::Simulate {
            command,
            history,
            print,
        }) => run_simulate(command, history, print),
        None => run_default(shell),
    }
}

fn run_init(shell: Shell) {
    match Init::new(shell).script() {
        Ok(script) => print!("{script}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run_default(shell: Shell) {
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

    let suggestions = pipeline().run(&attempt);
    show_selector(attempt.failed_command, suggestions);
}

fn run_simulate(command: String, history_path: Option<PathBuf>, print: bool) {
    let history = match history_path {
        Some(path) => match read_history_file(&path) {
            Ok(lines) => lines,
            Err(error) => {
                eprintln!("failed to read {}: {error}", path.display());
                std::process::exit(1);
            }
        },
        None => Vec::new(),
    };

    let attempt = Attempt::from_inputs(command, history);
    let suggestions = pipeline().run(&attempt);

    if print {
        print_suggestions(&suggestions);
        return;
    }

    show_selector(attempt.failed_command, suggestions);
}

fn pipeline() -> Pipeline {
    // Order matters: Path corrects the program first so subsequent passes can look up under the corrected program.
    // Completion uses authoritative sources (fish completions). Subcommand refines with user history.
    Pipeline::new(MAX_SUGGESTIONS)
        .with_completions(completions::detect())
        .add_pass(Pass::Path)
        .add_pass(Pass::Completion)
        .add_pass(Pass::Subcommand)
}

fn read_history_file(path: &PathBuf) -> std::io::Result<Vec<String>> {
    let contents = std::fs::read_to_string(path)?;
    let lines = contents
        .lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();
    Ok(lines)
}

fn print_suggestions(suggestions: &[Suggestion]) {
    if suggestions.is_empty() {
        return no_fugs_given();
    }

    for suggestion in suggestions {
        println!("{:.3}  {}", suggestion.score, suggestion.command);
    }
}

fn show_selector(failed_command: String, suggestions: Vec<Suggestion>) {
    if suggestions.is_empty() {
        return no_fugs_given();
    }

    let commands = suggestions
        .into_iter()
        .map(|suggestion| suggestion.command)
        .collect::<Vec<String>>();

    let Ok(selected_command) = Selector::new(failed_command, commands).show() else {
        return no_fugs_given();
    };

    println!("{selected_command}");
}

fn no_fugs_given() {
    println!("{NO_SUGGESTION_NEEDED}");
}
