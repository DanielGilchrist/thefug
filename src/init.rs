use crate::shell::{self, Shell};

use std::env::current_dir;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;

pub struct Init {
    shell: Shell,
}

impl Init {
    pub fn new(shell: Shell) -> Init {
        Init { shell }
    }

    pub fn init(&self) -> Result<(), std::io::Error> {
        Ok(())
    }

    pub fn init_dev(&self) -> Result<(), std::io::Error> {
        let mut root = current_dir()?;
        root.push("thefug");

        let script = self.determine_script();

        let mut file = File::create(&root)?;
        file.write_all(script.as_bytes())?;

        let mut perms = fs::metadata(&root)?.permissions();
        perms.set_mode(perms.mode() | 0o111);

        fs::set_permissions(root, perms)?;

        Ok(())
    }

    fn determine_script(&self) -> String {
        match self.shell.type_ {
            shell::Type::Bash => self.bash_script(),
            shell::Type::Fish => self.fish_script(),
            shell::Type::Zsh => self.zsh_script(),
            shell::Type::Unknown => unimplemented!(),
        }
    }

    fn bash_script(&self) -> String {
        String::from(
            "
#!/bin/bash

command=$(thefugbindev)

if [ \"$command\" = \"No fugs given.\" ]; then
  echo \"$command\"
else
  echo \"Running: $command\"
  eval \"$command\"
fi
            ",
        )
    }

    fn fish_script(&self) -> String {
        String::from(
            "
#!/bin/fish

set command (thefugbindev)

if test \"$command\" = \"No fugs given.\"
  echo \"$command\"
else
  echo \"Running: $command\"
  eval \"$command\"
end
",
        )
    }

    fn zsh_script(&self) -> String {
        String::from(
            "
#!/bin/zsh

command=$(thefugbindev)

if [ \"$command\" = \"No fugs given.\" ]; then
  echo \"$command\"
else
  echo \"Running: $command\"
  eval \"$command\"
fi
            ",
        )
    }
}
