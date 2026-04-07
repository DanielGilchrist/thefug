use crate::path_scan;
use crate::subcommand_scan;
use crate::suggestion::Suggestion;

pub enum Pass {
    Path,
    Subcommand { history: Vec<String> },
}

impl Pass {
    fn suggest(&self, command: &str) -> Vec<Suggestion> {
        match self {
            Pass::Path => path_scan::suggest(command),
            Pass::Subcommand { history } => subcommand_scan::suggest(command, history),
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

    pub fn run(&self, command: &str) -> Vec<Suggestion> {
        let mut suggestions = self
            .passes
            .iter()
            .flat_map(|pass| pass.suggest(command))
            .collect::<Vec<_>>();

        suggestions.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
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

    #[test]
    fn returns_empty_when_no_passes() {
        let pipeline = Pipeline::new(5);
        let suggestions = pipeline.run("gti");

        assert!(suggestions.is_empty());
    }

    #[test]
    fn sorts_by_similarity_descending() {
        let history = vec![
            "git pull", "git pull", "git push", "git push", "git log", "git log",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let pipeline = Pipeline::new(5).add_pass(Pass::Subcommand { history });
        let suggestions = pipeline.run("git pll");

        if suggestions.len() >= 2 {
            assert!(
                suggestions[0].similarity >= suggestions[1].similarity,
                "suggestions should be sorted by similarity descending"
            );
        }
    }

    #[test]
    fn truncates_to_max_suggestions() {
        let history = vec![
            "git pull", "git pull", "git push", "git push", "git log", "git log",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let pipeline = Pipeline::new(1).add_pass(Pass::Subcommand { history });
        let suggestions = pipeline.run("git pll");

        assert!(suggestions.len() <= 1);
    }

    #[test]
    fn deduplicates_by_command() {
        let history = vec!["git pull", "git pull", "git pull"]
            .into_iter()
            .map(String::from)
            .collect();

        let pipeline = Pipeline::new(5).add_pass(Pass::Subcommand { history });
        let suggestions = pipeline.run("git pll");
        let pull_count = suggestions
            .iter()
            .filter(|s| s.command == "git pull")
            .count();

        assert!(pull_count <= 1, "should not have duplicate suggestions");
    }
}
