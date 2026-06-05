pub struct Attempt {
    pub failed_command: String,
    pub history: Vec<String>,
}

impl Attempt {
    // history[0] is the command that invoked thefug; history[1] is the failed
    // command we want to correct. Both are stripped from history so passes
    // don't treat them as valid candidates.
    pub fn from_shell_history(history: Vec<String>) -> Option<Self> {
        let mut iter = history.into_iter();
        let invoker = iter.next()?;
        let failed_command = iter.next()?;
        let history = iter
            .filter(|entry| entry != &invoker && entry != &failed_command)
            .collect();

        Some(Self {
            failed_command,
            history,
        })
    }

    pub fn from_inputs(failed_command: String, history: Vec<String>) -> Self {
        Self {
            failed_command,
            history,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_shell_history_returns_none_for_short_history() {
        assert!(Attempt::from_shell_history(vec![]).is_none());
        assert!(Attempt::from_shell_history(vec!["fugd".into()]).is_none());
    }

    #[test]
    fn from_shell_history_extracts_correct_command() {
        let attempt = Attempt::from_shell_history(vec![
            "fugd".into(),
            "git puuulllll".into(),
            "git pull".into(),
            "ls".into(),
        ])
        .unwrap();

        assert_eq!(attempt.failed_command, "git puuulllll");
        assert_eq!(attempt.history, vec!["git pull", "ls"]);
    }

    #[test]
    fn from_shell_history_removes_all_copies_of_invoker_and_failed_command() {
        let attempt = Attempt::from_shell_history(vec![
            "./scripts/build-dev.sh && fugd".into(),
            "git puuulllll".into(),
            "./scripts/build-dev.sh && fugd".into(),
            "git puuulllll".into(),
            "./scripts/build-dev.sh && fugd".into(),
            "git puuulllll".into(),
            "git pull".into(),
            "git checkout main".into(),
        ])
        .unwrap();

        assert_eq!(attempt.failed_command, "git puuulllll");
        assert_eq!(attempt.history, vec!["git pull", "git checkout main"]);
    }

    #[test]
    fn from_shell_history_with_exactly_two_entries() {
        let attempt =
            Attempt::from_shell_history(vec!["fugd".into(), "gti status".into()]).unwrap();

        assert_eq!(attempt.failed_command, "gti status");
        assert!(attempt.history.is_empty());
    }

    #[test]
    fn from_shell_history_identical_invoker_and_failed_command() {
        let attempt = Attempt::from_shell_history(vec![
            "weird".into(),
            "weird".into(),
            "git pull".into(),
            "weird".into(),
        ])
        .unwrap();

        assert_eq!(attempt.failed_command, "weird");
        assert_eq!(attempt.history, vec!["git pull"]);
    }

    #[test]
    fn from_inputs_does_not_filter() {
        let attempt =
            Attempt::from_inputs("git pll".into(), vec!["git pll".into(), "git pull".into()]);

        assert_eq!(attempt.history, vec!["git pll", "git pull"]);
    }
}
