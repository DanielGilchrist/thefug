use crate::suggestion::Suggestion;

pub trait SuggestionPass {
    fn suggest(&self, command: &str) -> Vec<Suggestion>;
}

pub struct Pipeline {
    passes: Vec<Box<dyn SuggestionPass>>,
    max_suggestions: usize,
}

impl Pipeline {
    pub fn new(max_suggestions: usize) -> Self {
        Self {
            passes: Vec::new(),
            max_suggestions,
        }
    }

    pub fn add_pass(mut self, pass: Box<dyn SuggestionPass>) -> Self {
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

    struct FakePass {
        results: Vec<(String, f32)>,
    }

    impl SuggestionPass for FakePass {
        fn suggest(&self, _command: &str) -> Vec<Suggestion> {
            self.results
                .iter()
                .map(|(cmd, sim)| Suggestion::new(cmd.clone(), *sim))
                .collect()
        }
    }

    #[test]
    fn returns_empty_when_no_passes() {
        let pipeline = Pipeline::new(5);
        let suggestions = pipeline.run("gti");

        assert!(suggestions.is_empty());
    }

    #[test]
    fn collects_from_multiple_passes() {
        let pipeline = Pipeline::new(5)
            .add_pass(Box::new(FakePass {
                results: vec![("git".into(), 0.9)],
            }))
            .add_pass(Box::new(FakePass {
                results: vec![("gzip".into(), 0.5)],
            }));

        let suggestions = pipeline.run("gti");

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].command, "git");
        assert_eq!(suggestions[1].command, "gzip");
    }

    #[test]
    fn sorts_by_similarity_descending() {
        let pipeline = Pipeline::new(5).add_pass(Box::new(FakePass {
            results: vec![
                ("low".into(), 0.3),
                ("high".into(), 0.9),
                ("mid".into(), 0.6),
            ],
        }));

        let suggestions = pipeline.run("x");
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert_eq!(commands, vec!["high", "mid", "low"]);
    }

    #[test]
    fn truncates_to_max_suggestions() {
        let pipeline = Pipeline::new(2).add_pass(Box::new(FakePass {
            results: vec![("a".into(), 0.9), ("b".into(), 0.8), ("c".into(), 0.7)],
        }));

        let suggestions = pipeline.run("x");

        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn deduplicates_by_command() {
        let pipeline = Pipeline::new(5)
            .add_pass(Box::new(FakePass {
                results: vec![("git".into(), 0.9)],
            }))
            .add_pass(Box::new(FakePass {
                results: vec![("git".into(), 0.8)],
            }));

        let suggestions = pipeline.run("gti");
        let commands: Vec<&str> = suggestions.iter().map(|s| s.command.as_str()).collect();

        assert_eq!(commands, vec!["git"]);
    }
}
