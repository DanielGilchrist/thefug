use crate::attempt::Attempt;
use crate::completion_scan;
use crate::completions::{Completions, NoCompletions};
use crate::hypothesis::Hypothesis;
use crate::parsed_command::ParsedCommand;
use crate::path_scan;
use crate::subcommand_scan;
use crate::suggestion::Suggestion;

pub enum Pass {
    Path,
    Subcommand,
    Completion,
}

impl Pass {
    fn apply(
        &self,
        hypotheses: Vec<Hypothesis>,
        history: &[String],
        completions: &dyn Completions,
    ) -> Vec<Hypothesis> {
        match self {
            Pass::Path => path_scan::apply(hypotheses),
            Pass::Subcommand => subcommand_scan::apply(hypotheses, history),
            Pass::Completion => completion_scan::apply(hypotheses, completions),
        }
    }
}

pub struct Pipeline {
    passes: Vec<Pass>,
    completions: Box<dyn Completions>,
    max_suggestions: usize,
}

impl Pipeline {
    pub fn new(max_suggestions: usize) -> Self {
        Self {
            passes: Vec::new(),
            completions: Box::new(NoCompletions),
            max_suggestions,
        }
    }

    pub fn add_pass(mut self, pass: Pass) -> Self {
        self.passes.push(pass);
        self
    }

    pub fn with_completions(mut self, completions: Box<dyn Completions>) -> Self {
        self.completions = completions;
        self
    }

    pub fn run(&self, attempt: &Attempt) -> Vec<Suggestion> {
        let parsed = ParsedCommand::parse(&attempt.failed_command);
        let initial = vec![Hypothesis::from_parsed(&parsed)];

        let hypotheses = self.passes.iter().fold(initial, |hypotheses, pass| {
            pass.apply(hypotheses, &attempt.history, self.completions.as_ref())
        });

        let mut suggestions: Vec<Suggestion> = hypotheses
            .into_iter()
            .filter(|h| !h.matches_original(&parsed))
            .map(|h| Suggestion {
                command: h.to_command(),
                score: h.score,
            })
            .collect();

        suggestions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        suggestions.dedup_by(|a, b| a.command == b.command);
        suggestions.truncate(self.max_suggestions);

        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(failed_command: &str, history: Vec<&str>) -> Attempt {
        Attempt::from_inputs(
            failed_command.to_string(),
            history.into_iter().map(String::from).collect(),
        )
    }

    #[test]
    fn returns_empty_when_no_passes() {
        let pipeline = Pipeline::new(5);
        let suggestions = pipeline.run(&attempt("gti", vec![]));

        assert!(suggestions.is_empty());
    }

    #[test]
    fn sorts_by_score_descending() {
        let pipeline = Pipeline::new(5).add_pass(Pass::Subcommand);
        let suggestions = pipeline.run(&attempt(
            "git pll",
            vec![
                "git pull", "git pull", "git push", "git push", "git log", "git log",
            ],
        ));

        if suggestions.len() >= 2 {
            assert!(
                suggestions[0].score >= suggestions[1].score,
                "suggestions should be sorted by score descending"
            );
        }
    }

    #[test]
    fn truncates_to_max_suggestions() {
        let pipeline = Pipeline::new(1).add_pass(Pass::Subcommand);
        let suggestions = pipeline.run(&attempt(
            "git pll",
            vec![
                "git pull", "git pull", "git push", "git push", "git log", "git log",
            ],
        ));

        assert!(suggestions.len() <= 1);
    }

    #[test]
    fn deduplicates_by_command() {
        let pipeline = Pipeline::new(5).add_pass(Pass::Subcommand);
        let suggestions = pipeline.run(&attempt(
            "git pll",
            vec!["git pull", "git pull", "git pull"],
        ));
        let pull_count = suggestions
            .iter()
            .filter(|s| s.command == "git pull")
            .count();

        assert!(pull_count <= 1, "should not have duplicate suggestions");
    }

    #[test]
    fn drops_untransformed_hypotheses() {
        let pipeline = Pipeline::new(5).add_pass(Pass::Subcommand);
        let suggestions = pipeline.run(&attempt(
            "git status",
            vec!["git status", "git status", "git status"],
        ));

        assert!(
            suggestions.is_empty(),
            "no-op pass should produce no suggestions"
        );
    }
}
