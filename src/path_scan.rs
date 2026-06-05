use crate::hypothesis::Hypothesis;

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

pub fn apply(hypotheses: Vec<Hypothesis>) -> Vec<Hypothesis> {
    let executables = executables_on_path();
    apply_with_executables(hypotheses, &executables)
}

pub fn apply_with_executables(
    hypotheses: Vec<Hypothesis>,
    executables: &HashSet<String>,
) -> Vec<Hypothesis> {
    hypotheses
        .into_iter()
        .flat_map(|h| refine_one(h, executables))
        .collect()
}

fn refine_one(hypothesis: Hypothesis, executables: &HashSet<String>) -> Vec<Hypothesis> {
    if executables.contains(&hypothesis.program) {
        return vec![hypothesis];
    }

    let branches: Vec<Hypothesis> = executables
        .iter()
        .filter_map(|name| {
            let similarity = jaro_winkler(&hypothesis.program, name);
            if similarity < MIN_SIMILARITY {
                return None;
            }
            Some(Hypothesis {
                program: name.clone(),
                subcommand: hypothesis.subcommand.clone(),
                args: hypothesis.args.clone(),
                score: hypothesis.score * similarity as f32,
            })
        })
        .collect();

    if branches.is_empty() {
        vec![hypothesis]
    } else {
        branches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsed_command::ParsedCommand;

    fn executables() -> HashSet<String> {
        [
            "git", "grep", "cat", "cargo", "curl", "ls", "rm", "cp", "mv", "docker", "node",
            "npm",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn refine(raw: &str) -> Vec<Hypothesis> {
        let parsed = ParsedCommand::parse(raw);
        let initial = Hypothesis::from_parsed(&parsed);
        apply_with_executables(vec![initial], &executables())
    }

    fn commands_of(hypotheses: &[Hypothesis]) -> Vec<String> {
        hypotheses.iter().map(Hypothesis::to_command).collect()
    }

    #[test]
    fn passes_through_when_program_on_path() {
        let result = refine("git pull");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].program, "git");
        assert_eq!(result[0].score, 1.0, "pass-through must not change score");
    }

    #[test]
    fn branches_when_program_has_close_match() {
        let result = refine("gti");
        let commands = commands_of(&result);

        assert!(commands.contains(&"git".to_string()), "expected 'git' in {commands:?}");
        assert!(!commands.contains(&"gti".to_string()));
    }

    #[test]
    fn preserves_subcommand_and_args_when_branching() {
        let result = refine("gti status -v");

        let git = result.iter().find(|h| h.program == "git").expect("git branch");
        assert_eq!(git.subcommand.as_deref(), Some("status"));
        assert_eq!(git.args, "-v");
    }

    #[test]
    fn score_multiplies_by_program_similarity() {
        let result = refine("gti");
        let git = result.iter().find(|h| h.program == "git").expect("git branch");

        assert!(git.score < 1.0, "score should decay below 1.0");
        assert!(git.score > 0.5, "but stay above MIN_SIMILARITY");
    }

    #[test]
    fn passes_through_when_no_matches() {
        let result = refine("zzzzzzzzz arg");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].program, "zzzzzzzzz");
        assert_eq!(result[0].score, 1.0);
    }

    #[test]
    fn empty_input_passes_through_unchanged() {
        let parsed = ParsedCommand::parse("");
        let initial = Hypothesis::from_parsed(&parsed);
        let result = apply_with_executables(vec![initial], &executables());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].program, "");
    }
}
