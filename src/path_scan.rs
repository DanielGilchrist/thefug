use crate::suggestion::Suggestion;

use strsim::jaro_winkler;

use std::collections::HashSet;
use std::fs;

static MIN_SIMILARITY: f64 = 0.5;

fn executables_on_path() -> Vec<String> {
    let path_var = match std::env::var("PATH") {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let mut seen = HashSet::new();
    let mut executables = Vec::new();

    for dir in std::env::split_paths(&path_var) {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.filter_map(Result::ok) {
            let file_name = entry.file_name().to_string_lossy().into_owned();

            if seen.insert(file_name.clone()) {
                executables.push(file_name);
            }
        }
    }

    executables
}

pub fn suggest(command: &str) -> Vec<Suggestion> {
    let executables = executables_on_path();
    suggest_from_executables(command, &executables)
}

pub fn suggest_from_executables(command: &str, executables: &[String]) -> Vec<Suggestion> {
    let program = command.split_whitespace().next().unwrap_or(command);
    let rest: &str = command[program.len()..].trim_start();

    // If the program already exists on PATH, there's no typo in the program name
    if executables.iter().any(|e| e == program) {
        return Vec::new();
    }

    executables
        .iter()
        .filter(|name| name.as_str() != program)
        .filter_map(|name| {
            let similarity = jaro_winkler(program, name);

            if similarity >= MIN_SIMILARITY {
                let suggested_command = if rest.is_empty() {
                    name.clone()
                } else {
                    format!("{name} {rest}")
                };

                Some(Suggestion::new(suggested_command, similarity as f32))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn executables() -> Vec<String> {
        [
            "git", "grep", "cat", "cargo", "curl", "ls", "rm", "cp", "mv", "docker", "node",
            "npm",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[test]
    fn matches_typo_in_program_name() {
        let suggestions = suggest_from_executables("gti", &executables());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(commands.contains(&"git"), "expected 'git' in {commands:?}");
    }

    #[test]
    fn preserves_arguments() {
        let suggestions = suggest_from_executables("gti status", &executables());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"git status"),
            "expected 'git status' in {commands:?}"
        );
    }

    #[test]
    fn no_match_for_completely_different_input() {
        let suggestions = suggest_from_executables("zzzzzzzzz", &executables());

        assert!(suggestions.is_empty());
    }

    #[test]
    fn does_not_suggest_exact_match() {
        let suggestions = suggest_from_executables("git", &executables());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            !commands.contains(&"git"),
            "should not suggest the exact same program"
        );
    }

    #[test]
    fn matches_close_typo() {
        let suggestions = suggest_from_executables("carg", &executables());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"cargo"),
            "expected 'cargo' in {commands:?}"
        );
    }

    #[test]
    fn skips_when_program_exists() {
        let suggestions = suggest_from_executables("git pull", &executables());

        assert!(
            suggestions.is_empty(),
            "should not suggest when program exists on PATH"
        );
    }
}
