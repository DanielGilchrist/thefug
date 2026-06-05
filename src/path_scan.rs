use crate::parsed_command::ParsedCommand;
use crate::suggestion::Suggestion;

use strsim::jaro_winkler;

use std::collections::HashSet;
use std::fs;

static MIN_SIMILARITY: f64 = 0.5;

fn executables_on_path() -> HashSet<String> {
    let Ok(path_var) = std::env::var("PATH") else {
        return HashSet::new();
    };

    std::env::split_paths(&path_var)
        .filter_map(|dir| fs::read_dir(&dir).ok())
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

pub fn suggest(parsed: &ParsedCommand) -> Vec<Suggestion> {
    let executables = executables_on_path();
    suggest_from_executables(parsed, &executables)
}

pub fn suggest_from_executables(
    parsed: &ParsedCommand,
    executables: &HashSet<String>,
) -> Vec<Suggestion> {
    if executables.contains(parsed.program) {
        return Vec::new();
    }

    let rest = parsed.raw[parsed.program.len()..].trim_start();

    executables
        .iter()
        .filter_map(|name| {
            let similarity = jaro_winkler(parsed.program, name);
            if similarity < MIN_SIMILARITY {
                return None;
            }

            let command = if rest.is_empty() {
                name.clone()
            } else {
                format!("{name} {rest}")
            };

            Some(Suggestion {
                command,
                score: similarity as f32,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn executables() -> HashSet<String> {
        [
            "git", "grep", "cat", "cargo", "curl", "ls", "rm", "cp", "mv", "docker", "node",
            "npm",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn suggest_commands(command: &str) -> Vec<String> {
        let parsed = ParsedCommand::parse(command);
        suggest_from_executables(&parsed, &executables())
            .into_iter()
            .map(|s| s.command)
            .collect()
    }

    #[test]
    fn matches_typo_in_program_name() {
        let commands = suggest_commands("gti");

        assert!(commands.contains(&"git".to_string()), "expected 'git' in {commands:?}");
    }

    #[test]
    fn preserves_arguments() {
        let commands = suggest_commands("gti status");

        assert!(
            commands.contains(&"git status".to_string()),
            "expected 'git status' in {commands:?}"
        );
    }

    #[test]
    fn no_match_for_completely_different_input() {
        let commands = suggest_commands("zzzzzzzzz");

        assert!(commands.is_empty());
    }

    #[test]
    fn does_not_suggest_exact_match() {
        let commands = suggest_commands("git");

        assert!(commands.is_empty(), "should not suggest when program is on PATH");
    }

    #[test]
    fn matches_close_typo() {
        let commands = suggest_commands("carg");

        assert!(commands.contains(&"cargo".to_string()), "expected 'cargo' in {commands:?}");
    }

    #[test]
    fn skips_when_program_exists() {
        let commands = suggest_commands("git pull");

        assert!(
            commands.is_empty(),
            "should not suggest when program exists on PATH"
        );
    }
}
