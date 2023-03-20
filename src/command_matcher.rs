use ngrammatic::{Corpus, CorpusBuilder, Pad};

static MIN_SIMILARITY: f32 = 0.4;
static THRESHOLD: f32 = 0.25;

#[derive(Debug)]
pub struct Suggestion {
    pub command: String,
    pub similarity: f32,
}

impl Suggestion {
    pub fn new(command: String, similarity: f32) -> Self {
        Self {
            command,
            similarity,
        }
    }
}

pub struct CommandMatcher(Vec<String>);

impl CommandMatcher {
    pub fn new(commands: Vec<String>) -> Self {
        Self(commands)
    }

    pub fn find_match(&self, command: &str) -> Option<Vec<Suggestion>> {
        let corpus = self.build_corpus(command);
        let search_results = corpus.search(command, THRESHOLD);
        let suggestions = search_results
            .into_iter()
            .filter_map(|search_result| {
                if search_result.similarity >= MIN_SIMILARITY {
                    let suggestion = Suggestion::new(search_result.text, search_result.similarity);
                    Some(suggestion)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if suggestions.is_empty() {
            None
        } else {
            Some(suggestions)
        }
    }

    fn build_corpus(&self, command: &str) -> Corpus {
        let mut corpus = CorpusBuilder::new().arity(2).pad_full(Pad::Auto).finish();

        self.0.iter().for_each(|name| {
            if name != command {
                corpus.add_text(name)
            }
        });

        corpus
    }
}
