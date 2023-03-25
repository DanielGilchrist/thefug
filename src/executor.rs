use itertools::Itertools;
use std::{
    io,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

pub struct Executor;
impl Executor {
    pub fn execute_command(selected_command: &str) -> io::Result<()> {
        let mut args = selected_command.split_whitespace().collect_vec();
        let command_head = args.remove(0);

        let mut command = Command::new(command_head)
            .args(&args)
            .stdout(Stdio::piped())
            .spawn()?;

        {
            let stdout = command.stdout.as_mut().unwrap();
            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();

            for line in stdout_lines {
                println!("{:?}", line);
            }
        }

        command.wait()?;

        Ok(())
    }
}
