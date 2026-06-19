use crate::shell::{self, Shell};

use std::env;
use std::io;
use std::path::Path;

pub struct Init {
    shell: Shell,
}

impl Init {
    pub fn new(shell: Shell) -> Init {
        Init { shell }
    }

    pub fn script(&self) -> io::Result<String> {
        let bin = env::current_exe()?;
        Ok(self.script_for(&bin))
    }

    fn script_for(&self, bin: &Path) -> String {
        let bin = bin.display().to_string();
        match self.shell.type_ {
            shell::Type::Bash => bash_script(&bin),
            shell::Type::Zsh => zsh_script(&bin),
            shell::Type::Fish => fish_script(&bin),
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
    _fug_out=$(command "{bin}" bash)
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
    _fug_out=$(command "{bin}" zsh)
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
    set -l _fug_out (command "{bin}" fish)
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

    fn script_for(type_: shell::Type, bin: &str) -> String {
        init(type_).script_for(&PathBuf::from(bin))
    }

    #[test]
    fn bash_script_defines_fug_and_calls_absolute_path() {
        let s = script_for(shell::Type::Bash, "/opt/homebrew/bin/thefug");

        assert!(s.contains("fug()"));
        assert!(s.contains("/opt/homebrew/bin/thefug"));
        assert!(s.contains("history -a"));
        assert!(s.contains("\" bash)"));
    }

    #[test]
    fn zsh_script_uses_fc_w_to_flush_history() {
        let s = script_for(shell::Type::Zsh, "/usr/local/bin/thefug");

        assert!(s.contains("fug()"));
        assert!(s.contains("/usr/local/bin/thefug"));
        assert!(s.contains("fc -W"));
        assert!(s.contains("\" zsh)"));
    }

    #[test]
    fn fish_script_uses_function_syntax() {
        let s = script_for(shell::Type::Fish, "/usr/local/bin/thefug");

        assert!(s.contains("function fug"));
        assert!(s.contains("/usr/local/bin/thefug"));
        assert!(s.contains("end"));
        assert!(s.contains("\" fish)"));
    }

    #[test]
    fn bash_script_handles_no_fugs_response() {
        let s = script_for(shell::Type::Bash, "/x");

        assert!(s.contains("No fugs given."));
    }
}
