use crate::attempt::Attempt;
use crate::parsed_command::ParsedCommand;
use crate::path_scan;
use crate::subcommand_scan;
use crate::suggestion::Suggestion;

pub enum Pass {
    Path,
    Subcommand,
}

impl Pass {
    fn suggest(&self, parsed: &ParsedCommand, history: &[String]) -> Vec<Suggestion> {
        match self {
            Pass::Path => path_scan::suggest(parsed),
            Pass::Subcommand => subcommand_scan::suggest(parsed, history),
        }
    }
}

pub struct Pipeline {
    passes: Vec<Pass>,
    max_suggestions: usize,
}

impl Pipeline {
    pub fn new(max_suggestions: usize) -> Self {
        Self {
            passes: Vec::new(),
            max_suggestions,
        }
    }

    pub fn add_pass(mut self, pass: Pass) -> Self {
        self.passes.push(pass);
        self
    }

    pub fn run(&self, attempt: &Attempt) -> Vec<Suggestion> {
        let parsed = ParsedCommand::parse(&attempt.failed_command);

        let mut suggestions = self
            .passes
            .iter()
            .flat_map(|pass| pass.suggest(&parsed, &attempt.history))
            .collect::<Vec<_>>();

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
        let pull_count = suggestions.iter().filter(|s| s.command == "git pull").count();

        assert!(pull_count <= 1, "should not have duplicate suggestions");
    }
}
