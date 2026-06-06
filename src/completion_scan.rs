use crate::completions::Completions;
use crate::hypothesis::{Hypothesis, Subcommand};
use crate::similarity;

use std::collections::HashMap;

static MIN_SIMILARITY: f64 = 0.5;

// Hypotheses whose subcommand is a known-valid completion get a multiplicative
// boost so they outrank Path-only branches like `gsettings pll` that pass
// through unverified.
static VALIDATION_BOOST: f32 = 2.0;

pub fn apply(hypotheses: Vec<Hypothesis>, completions: &dyn Completions) -> Vec<Hypothesis> {
    let mut programs: Vec<&str> = hypotheses
        .iter()
        .filter(|h| matches!(h.subcommand, Subcommand::Original(_)))
        .map(|h| h.program.as_str())
        .collect();

    programs.sort();
    programs.dedup();

    let lookup = completions.subcommands_batch(&programs);

    hypotheses
        .into_iter()
        .flat_map(|h| refine_one(h, &lookup))
        .collect()
}

fn refine_one(hypothesis: Hypothesis, lookup: &HashMap<String, Vec<String>>) -> Vec<Hypothesis> {
    let Subcommand::Original(subcommand) = hypothesis.subcommand.clone() else {
        return vec![hypothesis];
    };

    let Some(candidates) = lookup.get(&hypothesis.program) else {
        return vec![hypothesis];
    };

    if candidates.is_empty() {
        return vec![hypothesis];
    }

    if candidates.iter().any(|c| c == &subcommand) {
        return vec![Hypothesis {
            score: hypothesis.score * VALIDATION_BOOST,
            subcommand: Subcommand::Corrected(subcommand),
            ..hypothesis
        }];
    }

    let branches: Vec<Hypothesis> = candidates
        .iter()
        .filter_map(|candidate| {
            let similarity = similarity::subcommand(&subcommand, candidate);
            if similarity < MIN_SIMILARITY {
                return None;
            }

            Some(Hypothesis {
                program: hypothesis.program.clone(),
                subcommand: Subcommand::Corrected(candidate.clone()),
                args: hypothesis.args.clone(),
                score: hypothesis.score * similarity as f32 * VALIDATION_BOOST,
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

    use std::collections::HashMap;

    struct StaticCompletions(HashMap<String, Vec<String>>);

    impl StaticCompletions {
        fn new(entries: &[(&str, &[&str])]) -> Self {
            let map = entries
                .iter()
                .map(|(prog, subs)| {
                    (
                        prog.to_string(),
                        subs.iter().map(|s| s.to_string()).collect(),
                    )
                })
                .collect();
            Self(map)
        }
    }

    impl Completions for StaticCompletions {
        fn subcommands(&self, program: &str) -> Vec<String> {
            self.0.get(program).cloned().unwrap_or_default()
        }
    }

    fn refine(raw: &str, completions: &dyn Completions) -> Vec<Hypothesis> {
        let parsed = ParsedCommand::parse(raw);
        let initial = Hypothesis::from_parsed(&parsed);
        apply(vec![initial], completions)
    }

    fn commands_of(hypotheses: &[Hypothesis]) -> Vec<String> {
        hypotheses.iter().map(Hypothesis::to_command).collect()
    }

    fn git_completions() -> StaticCompletions {
        StaticCompletions::new(&[("git", &["pull", "push", "status", "commit", "checkout"])])
    }

    #[test]
    fn corrects_typo_to_known_completion() {
        let commands = commands_of(&refine("git pll", &git_completions()));

        assert!(
            commands.contains(&"git pull".to_string()),
            "expected 'git pull' in {commands:?}"
        );
    }

    #[test]
    fn boosts_when_subcommand_is_valid_completion() {
        let result = refine("git pull", &git_completions());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subcommand.value(), Some("pull"));
        assert!(
            result[0].score > 1.0,
            "validated subcommands should be boosted above the seed score"
        );
    }

    #[test]
    fn passes_through_when_no_completions_for_program() {
        let result = refine("unknown cmd", &git_completions());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].program, "unknown");
        assert_eq!(result[0].subcommand.value(), Some("cmd"));
        assert_eq!(result[0].score, 1.0);
    }

    #[test]
    fn passes_through_when_no_subcommand() {
        let result = refine("git", &git_completions());

        assert_eq!(result.len(), 1);
        assert!(result[0].subcommand.value().is_none());
    }

    #[test]
    fn preserves_args_when_branching() {
        let result = refine("git pll origin main", &git_completions());
        let pull = result
            .iter()
            .find(|h| h.subcommand.value() == Some("pull"))
            .unwrap();

        assert_eq!(pull.args, "origin main");
    }

    #[test]
    fn score_reflects_similarity_and_validation_boost() {
        let result = refine("git pll", &git_completions());
        let pull = result
            .iter()
            .find(|h| h.subcommand.value() == Some("pull"))
            .unwrap();

        assert!(
            pull.score > 1.0,
            "corrected-to-completion branch should beat unverified pass-through"
        );
    }

    #[test]
    fn passes_through_when_no_candidates_meet_threshold() {
        let result = refine("git zzzzzzz", &git_completions());

        // No completion is similar enough to "zzzzzzz" → pass through.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subcommand.value(), Some("zzzzzzz"));
        assert_eq!(result[0].score, 1.0);
    }
}
