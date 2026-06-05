pub struct Suggestion {
    pub command: String,
    // Higher is better. Scales are not normalised across passes.
    pub score: f32,
}
