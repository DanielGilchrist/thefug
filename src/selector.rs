use promkit::{
    build::Builder,
    crossterm::style,
    register::Register,
    select::{self, State},
    selectbox::SelectBox,
    Prompt,
};
use std::io::Error;

pub struct Selector {
    command: String,
    commands: Vec<String>,
}

impl Selector {
    pub fn new(command: String, commands: Vec<String>) -> Self {
        Self { command, commands }
    }

    pub fn show(&self) -> Result<String, Error> {
        let mut selector = self.build_selector()?;
        selector.run()
    }

    fn build_selector(&self) -> promkit::Result<Prompt<State>> {
        let mut selectbox = Box::<SelectBox>::default();
        selectbox.register_all(&self.commands);

        let command = &self.command;
        select::Builder::default()
            .title(format!("The fug? ({command})"))
            .title_color(style::Color::DarkGreen)
            .selectbox(selectbox)
            .build()
    }
}
