pub enum Type {
    Bash,
    Fish,
    Zsh,
    Unknown,
}

pub struct Shell {
    pub type_: Type,
}

impl Shell {
    pub fn history_location(&self) -> Option<String> {
        match self.type_ {
            Type::Bash => self.with_home(".bash_history"),
            Type::Fish => self.with_home(".local/share/fish/fish_history"),
            Type::Zsh => self.with_home(".zsh_history"),
            Type::Unknown => None,
        }
    }

    fn with_home(&self, path: &str) -> Option<String> {
        let home = std::env::var("HOME").ok()?;
        Some(format!("{home}/{path}"))
    }

    fn determine_shell() -> Type {
        std::env::var("SHELL").map_or(Type::Unknown, |shell_output| {
            if shell_output.contains("bash") {
                Type::Bash
            } else if shell_output.contains("fish") {
                Type::Fish
            } else if shell_output.contains("zsh") {
                Type::Zsh
            } else {
                Type::Unknown
            }
        })
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            type_: Self::determine_shell(),
        }
    }
}
