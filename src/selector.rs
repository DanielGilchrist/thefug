use promkit::{
    build::Builder,
    crossterm::style,
    register::Register,
    select::{self, State},
    selectbox::SelectBox,
    Prompt,
};

pub struct Selector(Vec<String>);

impl Selector {
    pub fn new(commands: Vec<String>) -> Self {
        Self(commands)
    }

    pub fn show(&self) {
        let mut selector = match self.build_selector() {
            Ok(selector) => selector,
            Err(error) => {
                eprintln!("{:?}", error);
                return;
            }
        };

        match selector.run() {
            Ok(selected) => println!("Selected command: {:?}", selected),
            Err(error) => eprintln!("{:?}", error),
        };
    }

    fn build_selector(&self) -> promkit::Result<Prompt<State>> {
        let mut selectbox = Box::<SelectBox>::default();
        selectbox.register_all(&self.0);

        select::Builder::default()
            .title("The fug?")
            .title_color(style::Color::DarkGreen)
            .selectbox(selectbox)
            .build()
    }
}
