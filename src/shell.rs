#[derive(Debug, Clone)]
pub enum Type {
    Bash,
    Fish,
    Zsh,
}

#[derive(Debug, Clone)]
pub struct Shell {
    pub type_: Type,
}

impl Shell {
    // `$SHELL` is the login shell inherited from the parent process, not the shell
    // currently running, so it can't be trusted to identify the shell sourcing the
    // init script (e.g. `fish -c` started from a zsh parent reports `/bin/zsh`).
    // Callers that know their shell pass it explicitly via this constructor.
    pub fn from_name(name: &str) -> Option<Shell> {
        let type_ = match name {
            "bash" => Type::Bash,
            "fish" => Type::Fish,
            "zsh" => Type::Zsh,
            _ => return None,
        };

        Some(Shell { type_ })
    }

    pub fn history_location(&self) -> Option<String> {
        match self.type_ {
            Type::Bash => self.with_home(".bash_history"),
            Type::Fish => self.with_home(".local/share/fish/fish_history"),
            Type::Zsh => self.with_home(".zsh_history"),
        }
    }

    fn with_home(&self, path: &str) -> Option<String> {
        let home = std::env::var("HOME").ok()?;
        Some(format!("{home}/{path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_recognises_supported_shells() {
        assert!(matches!(
            Shell::from_name("bash"),
            Some(Shell { type_: Type::Bash })
        ));
        assert!(matches!(
            Shell::from_name("fish"),
            Some(Shell { type_: Type::Fish })
        ));
        assert!(matches!(
            Shell::from_name("zsh"),
            Some(Shell { type_: Type::Zsh })
        ));
    }

    #[test]
    fn from_name_rejects_unknown_shells() {
        assert!(Shell::from_name("powershell").is_none());
        assert!(Shell::from_name("/bin/zsh").is_none());
        assert!(Shell::from_name("").is_none());
    }
}
