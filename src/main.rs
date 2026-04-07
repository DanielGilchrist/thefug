mod history;
mod init;
mod path_scan;
mod pipeline;
mod selector;
mod shell;
mod subcommand_scan;
mod suggestion;

use crate::{
    history::History,
    init::Init,
    path_scan::PathPass,
    pipeline::Pipeline,
    selector::Selector,
    shell::Shell,
    subcommand_scan::SubcommandPass,
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

fn extract_failed_command(history: Vec<String>) -> Option<(String, Vec<String>)> {
    if history.len() < 2 {
        return None;
    }

    // History is most-recent-first:
    //   [0] = the command that invoked thefug (e.g. "fugd", "./scripts/build-dev.sh && fugd")
    //   [1] = the failed command we want to correct
    //   [2..] = older history
    let invoker = history[0].clone();
    let command = history[1].clone();

    // Remove ALL copies of the invoker and failed command from history
    // so passes don't see them as valid commands/subcommands
    let cleaned_history = history
        .into_iter()
        .filter(|entry| entry != &invoker && entry != &command)
        .collect();

    Some((command, cleaned_history))
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
            Ok(_) => println!("Successfully initialized dev environment!"),
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

    let (command, history) = match extract_failed_command(history) {
        Some(result) => result,
        None => return no_fugs_given(),
    };

    let pipeline = Pipeline::new(MAX_SUGGESTIONS)
        .add_pass(Box::new(SubcommandPass::new(history)))
        .add_pass(Box::new(PathPass::new()));
    let suggestions = pipeline.run(&command);

    if suggestions.is_empty() {
        return no_fugs_given();
    }

    let suggested_commands = suggestions
        .into_iter()
        .map(|suggestion| suggestion.command)
        .collect::<Vec<String>>();

    let Ok(selected_command) = Selector::new(command, suggested_commands).show() else {
        return no_fugs_given();
    };

    println!("{selected_command}");
}

fn no_fugs_given() {
    println!("{NO_SUGGESTION_NEEDED}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_returns_none_for_short_history() {
        assert!(extract_failed_command(vec![]).is_none());
        assert!(extract_failed_command(vec!["fugd".into()]).is_none());
    }

    #[test]
    fn extract_gets_correct_command() {
        let history = vec![
            "fugd".into(),
            "git puuulllll".into(),
            "git pull".into(),
            "ls".into(),
        ];
        let (command, _) = extract_failed_command(history).unwrap();
        assert_eq!(command, "git puuulllll");
    }

    #[test]
    fn extract_removes_all_copies_of_failed_command() {
        // Mimics real scenario: user retried the typo 3 times
        let history = vec![
            "./scripts/build-dev.sh && fugd".into(),
            "git puuulllll".into(),
            "./scripts/build-dev.sh && fugd".into(),
            "git puuulllll".into(),
            "./scripts/build-dev.sh && fugd".into(),
            "git puuulllll".into(),
            "git pull".into(),
            "git checkout main".into(),
        ];
        let (command, cleaned) = extract_failed_command(history).unwrap();

        assert_eq!(command, "git puuulllll");
        assert!(
            !cleaned.contains(&"git puuulllll".to_string()),
            "cleaned history should not contain the failed command: {cleaned:?}"
        );
        assert!(
            !cleaned.contains(&"./scripts/build-dev.sh && fugd".to_string()),
            "cleaned history should not contain the invoker: {cleaned:?}"
        );
        assert_eq!(cleaned, vec!["git pull", "git checkout main"]);
    }

    /// End-to-end test mimicking the exact real-world scenario from debug output.
    /// User typed `git puuulllll` 3 times, with `./scripts/build-dev.sh && fugd` between each.
    /// History also has git checkout (freq 32), git pull (freq 1), git pll (past typo, freq 1).
    #[test]
    fn end_to_end_repeated_typo_scenario() {
        let mut history: Vec<String> = Vec::new();

        // Most recent first (as fish parser returns)
        history.push("./scripts/build-dev.sh && fugd".into());
        history.push("git puuulllll".into());
        history.push("./scripts/build-dev.sh && fugd".into());
        history.push("git puuulllll".into());
        history.push("./scripts/build-dev.sh && fugd".into());
        history.push("git puuulllll".into());
        history.push("fugd".into());
        history.push("git pll".into());
        history.push("./scripts/build-dev.sh".into());
        history.push("cat README.md".into());

        // Older history with real git commands
        for _ in 0..32 {
            history.push("git checkout main".into());
        }
        history.push("git pull".into());
        history.push("git pull origin main".into());
        history.push("git push".into());
        history.push("git init".into());
        history.push("git commit -m 'fix'".into());
        history.push("git pu.ll".into()); // another past typo

        // Extract and run pipeline (without PathPass since we can't control $PATH in tests)
        let (command, cleaned_history) = extract_failed_command(history).unwrap();
        assert_eq!(command, "git puuulllll");

        let pipeline = Pipeline::new(MAX_SUGGESTIONS)
            .add_pass(Box::new(SubcommandPass::new(cleaned_history)));
        let suggestions = pipeline.run(&command);

        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            !suggestions.is_empty(),
            "should produce suggestions for 'git puuulllll'"
        );
        assert!(
            commands.contains(&"git pull"),
            "expected 'git pull' in suggestions: {commands:?}"
        );
    }
}
