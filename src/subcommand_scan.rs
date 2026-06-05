use crate::parsed_command::ParsedCommand;
use crate::suggestion::Suggestion;

use strsim::jaro_winkler;

use std::collections::HashMap;

static MIN_SIMILARITY: f64 = 0.5;
// At or above this, a subcommand is treated as established and not corrected.
static FREQUENT_SUBCOMMAND_THRESHOLD: usize = 2;
// Thresholds for the garbage filter: a freq-1 candidate is dropped if it's
// at least GARBAGE_SIMILARITY similar to another candidate with at least
// GARBAGE_NEIGHBOUR_FREQ occurrences (i.e. likely a past typo of it).
static GARBAGE_NEIGHBOUR_FREQ: usize = 5;
static GARBAGE_SIMILARITY: f64 = 0.8;

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

pub fn suggest(parsed: &ParsedCommand, history: &[String]) -> Vec<Suggestion> {
    let Some(subcommand) = parsed.subcommand else {
        return Vec::new();
    };

    let frequencies = subcommand_frequencies(history, parsed.program);

    if frequencies.get(subcommand).copied().unwrap_or(0) >= FREQUENT_SUBCOMMAND_THRESHOLD {
        return Vec::new();
    }

    frequencies
        .iter()
        .filter(|(candidate, _)| candidate.as_str() != subcommand)
        .filter(|(candidate, freq)| !is_likely_garbage(candidate, **freq, &frequencies))
        .filter_map(|(candidate, freq)| {
            let similarity = jaro_winkler(subcommand, candidate);
            if similarity < MIN_SIMILARITY {
                return None;
            }

            let score = similarity * (1.0 + *freq as f64).log2();
            let command = if parsed.args.is_empty() {
                format!("{} {}", parsed.program, candidate)
            } else {
                format!("{} {} {}", parsed.program, candidate, parsed.args)
            };

            Some(Suggestion {
                command,
                score: score as f32,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "git pll",
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

    fn suggest_commands(command: &str, history: &[String]) -> Vec<String> {
        let parsed = ParsedCommand::parse(command);
        suggest(&parsed, history)
            .into_iter()
            .map(|s| s.command)
            .collect()
    }

    #[test]
    fn corrects_subcommand_typo() {
        let commands = suggest_commands("git pll", &history());

        assert!(commands.contains(&"git pull".to_string()), "expected 'git pull' in {commands:?}");
    }

    #[test]
    fn preserves_trailing_arguments() {
        let commands = suggest_commands("git pll origin main", &history());

        assert!(
            commands.contains(&"git pull origin main".to_string()),
            "expected 'git pull origin main' in {commands:?}"
        );
    }

    #[test]
    fn skips_when_no_subcommand() {
        let commands = suggest_commands("git", &history());

        assert!(commands.is_empty());
    }

    #[test]
    fn skips_when_subcommand_is_frequent() {
        let commands = suggest_commands("git pull", &history());

        assert!(commands.is_empty(), "should not suggest when subcommand is already frequent");
    }

    #[test]
    fn filters_garbage_from_candidates() {
        let mut h = history();
        for _ in 0..5 {
            h.push("git pull".to_string());
        }

        let commands = suggest_commands("git plll", &h);

        assert!(
            !commands.contains(&"git pll".to_string()),
            "should not suggest past typos: {commands:?}"
        );
    }

    #[test]
    fn no_match_for_unrelated_subcommand() {
        let commands = suggest_commands("git zzzzz", &history());

        assert!(commands.is_empty());
    }

    #[test]
    fn works_for_other_programs() {
        let commands = suggest_commands("cargo tset", &history());

        assert!(
            commands.contains(&"cargo test".to_string()),
            "expected 'cargo test' in {commands:?}"
        );
    }

    #[test]
    fn falls_back_to_low_frequency_when_no_frequent_candidates() {
        let history = vec!["mycli foo".to_string(), "mycli bar".to_string()];
        let commands = suggest_commands("mycli fo", &history);

        assert!(
            commands.contains(&"mycli foo".to_string()),
            "should fall back to freq-1 candidates when no frequent ones exist: {commands:?}"
        );
    }

    #[test]
    fn no_suggestions_for_program_not_in_history() {
        let commands = suggest_commands("unknown cmd", &history());

        assert!(commands.is_empty());
    }

    #[test]
    fn works_with_deduplicated_history() {
        let history = vec![
            "git pull".to_string(),
            "git push".to_string(),
            "git status".to_string(),
            "git commit -m 'fix'".to_string(),
            "git log".to_string(),
        ];
        let commands = suggest_commands("git pll", &history);

        assert!(
            commands.contains(&"git pull".to_string()),
            "should still suggest 'git pull' even with all-freq-1 history: {commands:?}"
        );
    }

    #[test]
    fn corrects_very_mangled_typo() {
        let commands = suggest_commands("git puuulllll", &history());

        assert!(commands.contains(&"git pull".to_string()), "expected 'git pull' in {commands:?}");
    }

    #[test]
    fn frequent_candidate_ranks_higher() {
        let history = vec![
            "git pull".to_string(),
            "git pull".to_string(),
            "git pull".to_string(),
            "git pul".to_string(),
        ];
        let parsed = ParsedCommand::parse("git pll");
        let mut suggestions = suggest(&parsed, &history);
        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        assert!(suggestions.len() >= 2);
        assert_eq!(
            suggestions[0].command, "git pull",
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

        let commands = suggest_commands("git puuulllll", &history);

        assert!(
            commands.contains(&"git pull".to_string()),
            "should suggest 'git pull' even when other subcommands dominate history: {commands:?}"
        );
    }
}
