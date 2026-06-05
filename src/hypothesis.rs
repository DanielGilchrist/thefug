use crate::parsed_command::ParsedCommand;

// Each pass takes `Vec<Hypothesis>` and returns `Vec<Hypothesis>`. Returning
// the input unchanged means "no opinion"; returning N new ones replaces it
// with branches. Hypotheses identical to the original parsed command are
// dropped at the end of the pipeline — we never suggest the user's typed
// command back to them.
#[derive(Clone, Debug, PartialEq)]
pub struct Hypothesis {
    pub program: String,
    pub subcommand: Option<String>,
    pub args: String,
    pub score: f32,
}

impl Hypothesis {
    pub fn from_parsed(parsed: &ParsedCommand) -> Self {
        Self {
            program: parsed.program.to_string(),
            subcommand: parsed.subcommand.map(str::to_string),
            args: parsed.args.to_string(),
            score: 1.0,
        }
    }

    pub fn matches_original(&self, parsed: &ParsedCommand) -> bool {
        self.program == parsed.program
            && self.subcommand.as_deref() == parsed.subcommand
            && self.args == parsed.args
    }

    pub fn to_command(&self) -> String {
        match (&self.subcommand, self.args.is_empty()) {
            (Some(sub), false) => format!("{} {} {}", self.program, sub, self.args),
            (Some(sub), true) => format!("{} {}", self.program, sub),
            (None, false) => format!("{} {}", self.program, self.args),
            (None, true) => self.program.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_parsed_seeds_with_score_one() {
        let parsed = ParsedCommand::parse("git pull origin");
        let h = Hypothesis::from_parsed(&parsed);

        assert_eq!(h.program, "git");
        assert_eq!(h.subcommand.as_deref(), Some("pull"));
        assert_eq!(h.args, "origin");
        assert_eq!(h.score, 1.0);
    }

    #[test]
    fn matches_original_detects_pass_through() {
        let parsed = ParsedCommand::parse("git pull");
        let h = Hypothesis::from_parsed(&parsed);

        assert!(h.matches_original(&parsed));
    }

    #[test]
    fn matches_original_detects_transformation() {
        let parsed = ParsedCommand::parse("gti pll");
        let mut h = Hypothesis::from_parsed(&parsed);
        h.program = "git".to_string();

        assert!(!h.matches_original(&parsed));
    }

    #[test]
    fn to_command_program_only() {
        let h = Hypothesis {
            program: "git".to_string(),
            subcommand: None,
            args: String::new(),
            score: 1.0,
        };

        assert_eq!(h.to_command(), "git");
    }

    #[test]
    fn to_command_program_and_subcommand() {
        let h = Hypothesis {
            program: "git".to_string(),
            subcommand: Some("pull".to_string()),
            args: String::new(),
            score: 1.0,
        };

        assert_eq!(h.to_command(), "git pull");
    }

    #[test]
    fn to_command_with_args() {
        let h = Hypothesis {
            program: "git".to_string(),
            subcommand: Some("pull".to_string()),
            args: "origin main".to_string(),
            score: 1.0,
        };

        assert_eq!(h.to_command(), "git pull origin main");
    }

    #[test]
    fn to_command_program_with_args_no_subcommand() {
        let h = Hypothesis {
            program: "ls".to_string(),
            subcommand: None,
            args: "-la".to_string(),
            score: 1.0,
        };

        assert_eq!(h.to_command(), "ls -la");
    }
}
