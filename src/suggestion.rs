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
