use crate::shell::{self, Shell};

use std::env;
use std::fmt;
use std::io;
use std::path::Path;

pub struct Init {
    shell: Shell,
}

#[derive(Debug)]
pub enum Error {
    UnsupportedShell,
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnsupportedShell => write!(f, "unsupported shell"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl Init {
    pub fn new(shell: Shell) -> Init {
        Init { shell }
    }

    pub fn script(&self) -> Result<String, Error> {
        let bin = env::current_exe()?;
        self.script_for(&bin)
    }

    fn script_for(&self, bin: &Path) -> Result<String, Error> {
        let bin = bin.display();
        match self.shell.type_ {
            shell::Type::Bash => Ok(bash_script(&bin.to_string())),
            shell::Type::Zsh => Ok(zsh_script(&bin.to_string())),
            shell::Type::Fish => Ok(fish_script(&bin.to_string())),
            shell::Type::Unknown => Err(Error::UnsupportedShell),
        }
    }
}

// `history -a` flushes the current session's history to disk so thefug can
// read the failed command. `command` skips function/alias lookup.
fn bash_script(bin: &str) -> String {
    format!(
        r#"fug() {{
    history -a 2>/dev/null
    local _fug_out
    _fug_out=$(command "{bin}")
    if [ "$_fug_out" = "No fugs given." ]; then
        echo "$_fug_out"
    else
        echo "Running: $_fug_out"
        eval "$_fug_out"
    fi
}}
"#
    )
}

// `fc -W` is zsh's equivalent of bash's `history -a`.
fn zsh_script(bin: &str) -> String {
    format!(
        r#"fug() {{
    fc -W 2>/dev/null
    local _fug_out
    _fug_out=$(command "{bin}")
    if [ "$_fug_out" = "No fugs given." ]; then
        echo "$_fug_out"
    else
        echo "Running: $_fug_out"
        eval "$_fug_out"
    fi
}}
"#
    )
}

// Fish auto-saves history; no explicit flush needed.
fn fish_script(bin: &str) -> String {
    format!(
        r#"function fug
    set -l _fug_out (command "{bin}")
    if test "$_fug_out" = "No fugs given."
        echo "$_fug_out"
    else
        echo "Running: $_fug_out"
        eval "$_fug_out"
    end
end
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn init(type_: shell::Type) -> Init {
        Init::new(Shell { type_ })
    }

    fn script_for(type_: shell::Type, bin: &str) -> Result<String, Error> {
        init(type_).script_for(&PathBuf::from(bin))
    }

    #[test]
    fn bash_script_defines_fug_and_calls_absolute_path() {
        let s = script_for(shell::Type::Bash, "/opt/homebrew/bin/thefug").unwrap();

        assert!(s.contains("fug()"));
        assert!(s.contains("/opt/homebrew/bin/thefug"));
        assert!(s.contains("history -a"));
    }

    #[test]
    fn zsh_script_uses_fc_w_to_flush_history() {
        let s = script_for(shell::Type::Zsh, "/usr/local/bin/thefug").unwrap();

        assert!(s.contains("fug()"));
        assert!(s.contains("/usr/local/bin/thefug"));
        assert!(s.contains("fc -W"));
    }

    #[test]
    fn fish_script_uses_function_syntax() {
        let s = script_for(shell::Type::Fish, "/usr/local/bin/thefug").unwrap();

        assert!(s.contains("function fug"));
        assert!(s.contains("/usr/local/bin/thefug"));
        assert!(s.contains("end"));
    }

    #[test]
    fn unknown_shell_returns_error() {
        let result = script_for(shell::Type::Unknown, "/whatever");

        assert!(matches!(result, Err(Error::UnsupportedShell)));
    }

    #[test]
    fn bash_script_handles_no_fugs_response() {
        let s = script_for(shell::Type::Bash, "/x").unwrap();

        assert!(s.contains("No fugs given."));
    }
}
