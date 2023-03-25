use inquire::{error::InquireResult, Select};

pub struct Selector {
    command: String,
    commands: Vec<String>,
}

impl Selector {
    pub fn new(command: String, commands: Vec<String>) -> Self {
        Self { command, commands }
    }

    pub fn show(&self) -> InquireResult<String> {
        let command = &self.command;
        let commands = self.commands.clone();

        Select::new(&format!("The fug? ({command})"), commands).prompt()
    }
}
