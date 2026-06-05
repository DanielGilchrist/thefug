//! Fixtures use the same format as `thefug simulate --history <file>`, so any
//! failure can be reproduced interactively:
//!
//! ```text
//! thefug simulate --history tests/fixtures/<file> --print "<command>"
//! ```

use thefug::{
    attempt::Attempt,
    pipeline::{Pass, Pipeline},
    suggestion::Suggestion,
};

use std::fs;
use std::path::PathBuf;

const MAX_SUGGESTIONS: usize = 5;

fn load_fixture(name: &str) -> Vec<String> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);

    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
        .lines()
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect()
}

fn run(failed_command: &str, fixture: &str) -> Vec<Suggestion> {
    let attempt = Attempt::from_inputs(failed_command.to_string(), load_fixture(fixture));
    Pipeline::new(MAX_SUGGESTIONS)
        .add_pass(Pass::Path)
        .add_pass(Pass::Subcommand)
        .run(&attempt)
}

fn commands(suggestions: &[Suggestion]) -> Vec<&str> {
    suggestions.iter().map(|s| s.command.as_str()).collect()
}

#[test]
fn git_pull_suggested_for_severe_typo_amid_dominant_other_subcommand() {
    let suggestions = run("git puuulllll", "git_repeated_typo.txt");
    let commands = commands(&suggestions);

    assert!(
        commands.contains(&"git pull"),
        "expected 'git pull' even when 'git checkout main' dominates history: {commands:?}"
    );
}

#[test]
fn cargo_test_suggested_for_subcommand_typo() {
    let suggestions = run("cargo tset", "cargo_subcommand.txt");
    let commands = commands(&suggestions);

    assert!(
        commands.contains(&"cargo test"),
        "expected 'cargo test' in {commands:?}"
    );
}

#[test]
fn cargo_build_suggested_for_subcommand_typo() {
    let suggestions = run("cargo buidl", "cargo_subcommand.txt");
    let commands = commands(&suggestions);

    assert!(
        commands.contains(&"cargo build"),
        "expected 'cargo build' in {commands:?}"
    );
}

#[test]
fn arguments_preserved_through_pipeline() {
    let suggestions = run("git pll origin main", "sparse_history.txt");
    let commands = commands(&suggestions);

    assert!(
        commands.contains(&"git pull origin main"),
        "expected args preserved in {commands:?}"
    );
}

#[test]
fn frequent_correction_outranks_rare_one() {
    let suggestions = run("cargo tst", "cargo_subcommand.txt");

    assert!(!suggestions.is_empty(), "expected at least one suggestion");
    assert_eq!(
        suggestions[0].command,
        "cargo test",
        "higher-frequency candidate should rank first; got {:?}",
        commands(&suggestions)
    );
}

#[test]
fn composes_path_and_subcommand_passes_for_double_typo() {
    let suggestions = run("gti pll", "git_repeated_typo.txt");
    let commands = commands(&suggestions);

    assert!(
        commands.contains(&"git pull"),
        "double-typo should resolve to 'git pull' via Path + Subcommand composition: {commands:?}"
    );
}

#[test]
fn no_suggestions_for_unrelated_program() {
    // PATH pass may match something on the host; we only assert no `zzz <sub>`.
    let suggestions = run("zzz somethign", "sparse_history.txt");

    assert!(
        !suggestions.iter().any(|s| s.command.starts_with("zzz ")),
        "subcommand pass should not invent a `zzz` correction: {:?}",
        commands(&suggestions)
    );
}
