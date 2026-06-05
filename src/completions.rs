use std::collections::HashMap;
use std::process::Command;

pub trait Completions {
    fn subcommands(&self, program: &str) -> Vec<String>;

    // Batch query so impls that shell out (e.g. fish) can amortise process
    // startup across many programs in one call.
    fn subcommands_batch(&self, programs: &[&str]) -> HashMap<String, Vec<String>> {
        programs
            .iter()
            .map(|&p| (p.to_string(), self.subcommands(p)))
            .collect()
    }
}

pub struct NoCompletions;

impl Completions for NoCompletions {
    fn subcommands(&self, _program: &str) -> Vec<String> {
        Vec::new()
    }
}

pub struct FishCompletions;

// Refuse anything that could break shell quoting in the batch script.
fn is_safe_program_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
}

impl Completions for FishCompletions {
    fn subcommands(&self, program: &str) -> Vec<String> {
        self.subcommands_batch(&[program])
            .remove(program)
            .unwrap_or_default()
    }

    fn subcommands_batch(&self, programs: &[&str]) -> HashMap<String, Vec<String>> {
        let safe: Vec<&str> = programs
            .iter()
            .copied()
            .filter(|p| is_safe_program_name(p))
            .collect();

        if safe.is_empty() {
            return HashMap::new();
        }

        query_fish_batch(&safe)
    }
}

const SECTION_MARKER: &str = "__THEFUG_SECTION__";

fn query_fish_batch(programs: &[&str]) -> HashMap<String, Vec<String>> {
    let script = programs
        .iter()
        .map(|p| format!("echo '{SECTION_MARKER}{p}'\ncomplete -C '{p} '"))
        .collect::<Vec<_>>()
        .join("\n");

    let output = Command::new("fish").args(["-c", &script]).output().ok();

    let Some(output) = output else {
        return HashMap::new();
    };

    if !output.status.success() {
        return HashMap::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_batch_output(&stdout)
}

fn parse_batch_output(stdout: &str) -> HashMap<String, Vec<String>> {
    let mut sections: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;
    let mut section_text = String::new();

    for line in stdout.lines() {
        if let Some(name) = line.strip_prefix(SECTION_MARKER) {
            if let Some(prev) = current.take() {
                sections.insert(prev, parse_fish_output(&section_text));
                section_text.clear();
            }
            current = Some(name.to_string());
        } else if current.is_some() {
            section_text.push_str(line);
            section_text.push('\n');
        }
    }

    if let Some(name) = current {
        sections.insert(name, parse_fish_output(&section_text));
    }

    sections
}

// Fish prints `<subcommand>\t<description>` per line. Two fallback modes
// we need to reject:
//
//   1. Unknown program: fish lists files in cwd (no tab in any line).
//   2. "Wrapper" programs like `time`, `sudo`, `nohup`: fish completes
//      with any executable on PATH and tags the description `command` /
//      `command link`. These aren't real subcommands.
fn parse_fish_output(stdout: &str) -> Vec<String> {
    let lines: Vec<&str> = stdout.lines().collect();

    if !lines.iter().any(|l| l.contains('\t')) {
        return Vec::new();
    }

    let parsed: Vec<(&str, &str)> = lines.iter().filter_map(|l| l.split_once('\t')).collect();

    let command_like = parsed
        .iter()
        .filter(|(_, d)| {
            d.eq_ignore_ascii_case("command") || d.eq_ignore_ascii_case("command link")
        })
        .count();

    if command_like * 2 > parsed.len() {
        return Vec::new();
    }

    parsed.into_iter().map(|(n, _)| n.to_string()).collect()
}

pub fn detect() -> Box<dyn Completions> {
    let fish_available = Command::new("fish")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if fish_available {
        Box::new(FishCompletions)
    } else {
        Box::new(NoCompletions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_completions_returns_empty() {
        assert!(NoCompletions.subcommands("git").is_empty());
    }

    #[test]
    fn parses_subcommand_lines_with_descriptions() {
        let stdout = "add\tAdd file contents\npull\tFetch and merge\npush\tUpdate remote refs\n";

        assert_eq!(parse_fish_output(stdout), vec!["add", "pull", "push"]);
    }

    #[test]
    fn discards_file_completion_fallback() {
        // No tabs → fish was listing files in cwd, not real completions.
        let stdout = "Cargo.lock\nCargo.toml\nREADME.md\n";

        assert!(parse_fish_output(stdout).is_empty());
    }

    #[test]
    fn keeps_alias_entries() {
        let stdout = "build\tCompile a local package\nb\talias: build\n";

        assert_eq!(parse_fish_output(stdout), vec!["build", "b"]);
    }

    #[test]
    fn empty_output_returns_empty() {
        assert!(parse_fish_output("").is_empty());
    }

    #[test]
    fn discards_wrapper_command_completion_fallback() {
        // What fish returns for `time `, `sudo `, etc. mostly executables
        // on PATH with description "command".
        let stdout = "aa\tcommand\nab\tcommand\nls\tcommand link\nLABEL\talias LABEL=foo\n";

        assert!(parse_fish_output(stdout).is_empty());
    }

    #[test]
    fn keeps_real_subcommands_with_few_command_entries() {
        // A handful of "command" entries mixed with real subcommands shouldn't
        // wipe out the legitimate ones.
        let stdout = "pull\tFetch and merge\npush\tUpdate remote refs\naa\tcommand\n";

        assert_eq!(parse_fish_output(stdout), vec!["pull", "push", "aa"]);
    }

    #[test]
    fn parse_batch_output_splits_on_section_marker() {
        let stdout = format!(
            "{SECTION_MARKER}git\nadd\tAdd files\npull\tFetch\n{SECTION_MARKER}cargo\nbuild\tCompile\n"
        );
        let sections = parse_batch_output(&stdout);

        assert_eq!(
            sections.get("git"),
            Some(&vec!["add".into(), "pull".into()])
        );
        assert_eq!(sections.get("cargo"), Some(&vec!["build".into()]));
    }

    #[test]
    fn is_safe_program_name_accepts_real_program_names_and_rejects_shell_injection() {
        assert!(is_safe_program_name("git"));
        assert!(is_safe_program_name("cargo-edit"));
        assert!(is_safe_program_name("g++"));
        assert!(!is_safe_program_name("weird'name"));
        assert!(!is_safe_program_name("path with space"));
        assert!(!is_safe_program_name(""));
    }
}
