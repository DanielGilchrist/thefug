pub struct ParsedCommand<'a> {
    pub raw: &'a str,
    pub program: &'a str,
    pub subcommand: Option<&'a str>,
    pub args: &'a str,
}

impl<'a> ParsedCommand<'a> {
    pub fn parse(raw: &'a str) -> Self {
        let (program, rest) = split_first_word(raw);
        let (subcommand, args) = match split_first_word(rest) {
            ("", _) => (None, ""),
            (sub, rest) => (Some(sub), rest),
        };

        Self {
            raw,
            program,
            subcommand,
            args,
        }
    }
}

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        Some(idx) => (&s[..idx], s[idx..].trim_start()),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let parsed = ParsedCommand::parse("");

        assert_eq!(parsed.program, "");
        assert_eq!(parsed.subcommand, None);
        assert_eq!(parsed.args, "");
    }

    #[test]
    fn whitespace_only() {
        let parsed = ParsedCommand::parse("   ");

        assert_eq!(parsed.program, "");
        assert_eq!(parsed.subcommand, None);
        assert_eq!(parsed.args, "");
    }

    #[test]
    fn program_only() {
        let parsed = ParsedCommand::parse("git");

        assert_eq!(parsed.program, "git");
        assert_eq!(parsed.subcommand, None);
        assert_eq!(parsed.args, "");
    }

    #[test]
    fn program_and_subcommand() {
        let parsed = ParsedCommand::parse("git pull");

        assert_eq!(parsed.program, "git");
        assert_eq!(parsed.subcommand, Some("pull"));
        assert_eq!(parsed.args, "");
    }

    #[test]
    fn program_subcommand_and_args() {
        let parsed = ParsedCommand::parse("git pull origin main");

        assert_eq!(parsed.program, "git");
        assert_eq!(parsed.subcommand, Some("pull"));
        assert_eq!(parsed.args, "origin main");
    }

    #[test]
    fn leading_and_trailing_whitespace() {
        let parsed = ParsedCommand::parse("   git   pull   origin   ");

        assert_eq!(parsed.program, "git");
        assert_eq!(parsed.subcommand, Some("pull"));
        assert_eq!(parsed.args, "origin   ");
    }

    #[test]
    fn raw_is_preserved() {
        let raw = "  git pull  ";
        let parsed = ParsedCommand::parse(raw);

        assert_eq!(parsed.raw, raw);
    }
}
