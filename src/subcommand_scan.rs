use crate::hypothesis::Hypothesis;

use strsim::jaro_winkler;

use std::collections::HashMap;

static MIN_SIMILARITY: f64 = 0.5;
// At or above this, a subcommand is treated as established and the
// hypothesis is passed through unchanged.
static FREQUENT_SUBCOMMAND_THRESHOLD: usize = 2;
// Thresholds for the garbage filter: a freq-1 candidate is dropped if it's
// at least GARBAGE_SIMILARITY similar to another candidate with at least
// GARBAGE_NEIGHBOUR_FREQ occurrences (i.e. likely a past typo of it).
static GARBAGE_NEIGHBOUR_FREQ: usize = 5;
static GARBAGE_SIMILARITY: f64 = 0.8;

pub fn apply(hypotheses: Vec<Hypothesis>, history: &[String]) -> Vec<Hypothesis> {
    hypotheses
        .into_iter()
        .flat_map(|h| refine_one(h, history))
        .collect()
}

fn refine_one(hypothesis: Hypothesis, history: &[String]) -> Vec<Hypothesis> {
    let Some(subcommand) = hypothesis.subcommand.clone() else {
        return vec![hypothesis];
    };

    let frequencies = subcommand_frequencies(history, &hypothesis.program);

    if frequencies.get(&subcommand).copied().unwrap_or(0) >= FREQUENT_SUBCOMMAND_THRESHOLD {
        return vec![hypothesis];
    }

    let branches: Vec<Hypothesis> = frequencies
        .iter()
        .filter(|(candidate, _)| candidate.as_str() != subcommand)
        .filter(|(candidate, freq)| !is_likely_garbage(candidate, **freq, &frequencies))
        .filter_map(|(candidate, freq)| {
            let similarity = jaro_winkler(&subcommand, candidate);
            if similarity < MIN_SIMILARITY {
                return None;
            }
            let freq_boost = (1.0 + *freq as f64).log2();
            Some(Hypothesis {
                program: hypothesis.program.clone(),
                subcommand: Some(candidate.clone()),
                args: hypothesis.args.clone(),
                score: hypothesis.score * (similarity * freq_boost) as f32,
            })
        })
        .collect();

    if branches.is_empty() {
        vec![hypothesis]
    } else {
        branches
    }
}

fn subcommand_frequencies(history: &[String], program: &str) -> HashMap<String, usize> {
    let mut frequencies: HashMap<String, usize> = HashMap::new();

    for entry in history {
        let mut parts = entry.splitn(2, char::is_whitespace);
        let entry_program = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("").trim();

        if entry_program != program || rest.is_empty() {
            continue;
        }

        let subcommand = rest.split_whitespace().next().unwrap_or(rest);
        *frequencies.entry(subcommand.to_string()).or_insert(0) += 1;
    }

    frequencies
}

fn is_likely_garbage(candidate: &str, freq: usize, frequencies: &HashMap<String, usize>) -> bool {
    if freq > 1 {
        return false;
    }

    frequencies.iter().any(|(other, &other_freq)| {
        other.as_str() != candidate
            && other_freq >= GARBAGE_NEIGHBOUR_FREQ
            && jaro_winkler(candidate, other) > GARBAGE_SIMILARITY
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsed_command::ParsedCommand;

    fn history() -> Vec<String> {
        [
            "git pull",
            "git pull",
            "git pull origin main",
            "git push",
            "git push",
            "git push origin main",
            "git status",
            "git status",
            "git status",
            "git commit -m 'fix'",
            "git commit -m 'update'",
            "git log",
            "git log",
            "git pll", // past typo (freq 1)
            "cargo build",
            "cargo build",
            "cargo test",
            "cargo test",
            "cargo clippy",
            "cargo clippy",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    fn refine(raw: &str, history: &[String]) -> Vec<Hypothesis> {
        let parsed = ParsedCommand::parse(raw);
        let initial = Hypothesis::from_parsed(&parsed);
        apply(vec![initial], history)
    }

    fn commands_of(hypotheses: &[Hypothesis]) -> Vec<String> {
        hypotheses.iter().map(Hypothesis::to_command).collect()
    }

    #[test]
    fn corrects_subcommand_typo() {
        let commands = commands_of(&refine("git pll", &history()));

        assert!(
            commands.contains(&"git pull".to_string()),
            "expected 'git pull' in {commands:?}"
        );
    }

    #[test]
    fn preserves_trailing_arguments() {
        let commands = commands_of(&refine("git pll origin main", &history()));

        assert!(
            commands.contains(&"git pull origin main".to_string()),
            "expected 'git pull origin main' in {commands:?}"
        );
    }

    #[test]
    fn passes_through_when_no_subcommand() {
        let result = refine("git", &history());

        assert_eq!(result.len(), 1);
        assert!(result[0].subcommand.is_none());
        assert_eq!(result[0].score, 1.0);
    }

    #[test]
    fn passes_through_when_subcommand_is_established() {
        let result = refine("git pull", &history());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subcommand.as_deref(), Some("pull"));
        assert_eq!(
            result[0].score, 1.0,
            "established subcommand must not change score"
        );
    }

    #[test]
    fn filters_garbage_from_candidates() {
        let mut h = history();
        for _ in 0..5 {
            h.push("git pull".to_string());
        }
        let commands = commands_of(&refine("git plll", &h));

        assert!(
            !commands.contains(&"git pll".to_string()),
            "should not suggest past typos: {commands:?}"
        );
    }

    #[test]
    fn passes_through_when_no_candidates() {
        let result = refine("git zzzzz", &history());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].subcommand.as_deref(), Some("zzzzz"));
        assert_eq!(result[0].score, 1.0);
    }

    #[test]
    fn works_for_other_programs() {
        let commands = commands_of(&refine("cargo tset", &history()));

        assert!(
            commands.contains(&"cargo test".to_string()),
            "expected 'cargo test' in {commands:?}"
        );
    }

    #[test]
    fn falls_back_to_low_frequency_when_no_frequent_candidates() {
        let history = vec!["mycli foo".to_string(), "mycli bar".to_string()];
        let commands = commands_of(&refine("mycli fo", &history));

        assert!(
            commands.contains(&"mycli foo".to_string()),
            "should fall back to freq-1 candidates when no frequent ones exist: {commands:?}"
        );
    }

    #[test]
    fn passes_through_when_program_not_in_history() {
        let result = refine("unknown cmd", &history());

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].program, "unknown");
        assert_eq!(result[0].subcommand.as_deref(), Some("cmd"));
    }

    #[test]
    fn corrects_very_mangled_typo() {
        let commands = commands_of(&refine("git puuulllll", &history()));

        assert!(
            commands.contains(&"git pull".to_string()),
            "expected 'git pull' in {commands:?}"
        );
    }

    #[test]
    fn frequent_candidate_ranks_higher() {
        let history = vec![
            "git pull".to_string(),
            "git pull".to_string(),
            "git pull".to_string(),
            "git pul".to_string(),
        ];
        let mut result = refine("git pll", &history);
        result.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        assert!(result.len() >= 2);
        assert_eq!(
            result[0].to_command(),
            "git pull",
            "higher frequency candidate should rank first"
        );
    }

    #[test]
    fn works_with_mixed_frequency_history() {
        let mut history: Vec<String> = Vec::new();
        for _ in 0..32 {
            history.push("git checkout main".to_string());
        }
        history.push("git pull".to_string());
        history.push("git pll".to_string());
        let commands = commands_of(&refine("git puuulllll", &history));

        assert!(
            commands.contains(&"git pull".to_string()),
            "should suggest 'git pull' even when other subcommands dominate history: {commands:?}"
        );
    }
}
