use crate::suggestion::Suggestion;

use strsim::jaro_winkler;

use std::collections::HashMap;

static MIN_SIMILARITY: f64 = 0.5;

/// Build a frequency map of subcommands seen in history for a given program.
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

/// Determine if a candidate is likely a past typo (garbage).
/// A candidate is garbage if it has frequency 1 AND is very similar
/// to another candidate with much higher frequency.
fn is_likely_garbage(candidate: &str, freq: usize, frequencies: &HashMap<String, usize>) -> bool {
    if freq > 1 {
        return false;
    }

    frequencies.iter().any(|(other, &other_freq)| {
        other.as_str() != candidate && other_freq >= 5 && jaro_winkler(candidate, other) > 0.8
    })
}

pub fn suggest(command: &str, history: &[String]) -> Vec<Suggestion> {
    let mut parts = command.splitn(2, char::is_whitespace);
    let program = parts.next().unwrap_or(command);
    let rest = match parts.next() {
        Some(rest) if !rest.trim().is_empty() => rest.trim(),
        _ => return Vec::new(),
    };

    let subcommand = rest.split_whitespace().next().unwrap_or(rest);
    let args = rest[subcommand.len()..].trim_start();

    let frequencies = subcommand_frequencies(history, program);

    // If the subcommand itself is frequent, it's probably not a typo
    if frequencies.get(subcommand).copied().unwrap_or(0) >= 2 {
        return Vec::new();
    }

    frequencies
        .iter()
        .filter(|(candidate, _)| candidate.as_str() != subcommand)
        .filter(|(candidate, freq)| !is_likely_garbage(candidate, **freq, &frequencies))
        .filter_map(|(candidate, freq)| {
            let freq = *freq;
            let similarity = jaro_winkler(subcommand, candidate);

            if similarity >= MIN_SIMILARITY {
                // Boost score with frequency: similarity * log2(1 + freq)
                let score = similarity * (1.0 + freq as f64).log2();

                let suggested_command = if args.is_empty() {
                    format!("{program} {candidate}")
                } else {
                    format!("{program} {candidate} {args}")
                };

                Some(Suggestion::new(suggested_command, score as f32))
            } else {
                None
            }
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

    #[test]
    fn corrects_subcommand_typo() {
        let suggestions = suggest("git pll", &history());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"git pull"),
            "expected 'git pull' in {commands:?}"
        );
    }

    #[test]
    fn preserves_trailing_arguments() {
        let suggestions = suggest("git pll origin main", &history());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"git pull origin main"),
            "expected 'git pull origin main' in {commands:?}"
        );
    }

    #[test]
    fn skips_when_no_subcommand() {
        let suggestions = suggest("git", &history());

        assert!(suggestions.is_empty());
    }

    #[test]
    fn skips_when_subcommand_is_frequent() {
        let suggestions = suggest("git pull", &history());

        assert!(
            suggestions.is_empty(),
            "should not suggest when subcommand is already frequent"
        );
    }

    #[test]
    fn filters_garbage_from_candidates() {
        // "pll" is freq 1 and very similar to "pull" which has freq 5+
        let mut h = history();
        for _ in 0..5 {
            h.push("git pull".to_string());
        }

        let suggestions = suggest("git plll", &h);
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            !commands.contains(&"git pll"),
            "should not suggest past typos: {commands:?}"
        );
    }

    #[test]
    fn no_match_for_unrelated_subcommand() {
        let suggestions = suggest("git zzzzz", &history());

        assert!(suggestions.is_empty());
    }

    #[test]
    fn works_for_other_programs() {
        let suggestions = suggest("cargo tset", &history());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"cargo test"),
            "expected 'cargo test' in {commands:?}"
        );
    }

    #[test]
    fn falls_back_to_low_frequency_when_no_frequent_candidates() {
        let history = vec!["mycli foo".to_string(), "mycli bar".to_string()];
        let suggestions = suggest("mycli fo", &history);
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"mycli foo"),
            "should fall back to freq-1 candidates when no frequent ones exist: {commands:?}"
        );
    }

    #[test]
    fn no_suggestions_for_program_not_in_history() {
        let suggestions = suggest("unknown cmd", &history());

        assert!(suggestions.is_empty());
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
        let suggestions = suggest("git pll", &history);
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"git pull"),
            "should still suggest 'git pull' even with all-freq-1 history: {commands:?}"
        );
    }

    #[test]
    fn corrects_very_mangled_typo() {
        let suggestions = suggest("git puuulllll", &history());
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"git pull"),
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
        let mut suggestions = suggest("git pll", &history);
        suggestions.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());

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

        let suggestions = suggest("git puuulllll", &history);
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert!(
            commands.contains(&"git pull"),
            "should suggest 'git pull' even when other subcommands dominate history: {commands:?}"
        );
    }
}
