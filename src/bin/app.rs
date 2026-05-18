use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{DefaultTerminal, Frame};
use std::thread::sleep;

use crate::{
    bye::{BYE_TIME, Bye},
    logkeys::LogKeys,
};

pub trait Screen {
    fn handle_input(&mut self, key: KeyCode) -> Option<Box<dyn Screen>>;
    fn render(&mut self, f: &mut Frame);
}

mod helper {
    pub fn validate_field(field: &str) -> bool {
        field.trim().is_empty()
    }
}

mod logkeys {
    use std::process::exit;

    use crossterm::event::KeyCode;
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        style::{Style, Stylize},
        widgets::{Block, Borders, Paragraph},
    };

    use crate::{Screen, helper::validate_field};

    #[derive(Clone)]
    pub enum Field {
        PublicKey,
        PrivateKey,
    }

    #[derive(Clone)]
    pub struct LogKeys {
        pub public_key_content: String,
        pub private_key_content: String,
        pub field: Field,
    }

    impl LogKeys {
        pub fn new() -> Self {
            Self {
                public_key_content: String::new(),
                private_key_content: String::new(),
                field: Field::PublicKey,
            }
        }
    }

    impl Screen for LogKeys {
        fn handle_input(&mut self, key: crossterm::event::KeyCode) -> Option<Box<dyn Screen>> {
            match key {
                KeyCode::Esc => {
                    exit(0);
                }

                KeyCode::Char(c) => match self.field {
                    Field::PublicKey => {
                        self.public_key_content.push(c);
                    }
                    Field::PrivateKey => {
                        self.private_key_content.push(c);
                    }
                },

                KeyCode::Enter => {
                    if validate_field(&self.public_key_content) {
                        self.public_key_content = "public key is required".to_string();
                        return Some(Box::new(self.clone()));
                    }

                    if validate_field(&self.private_key_content) {
                        self.private_key_content = "private key is required".to_string();
                        return Some(Box::new(self.clone()));
                    }

                    return Some(Box::new(self.clone()));
                }

                KeyCode::Backspace => match self.field {
                    Field::PublicKey => {
                        self.public_key_content.pop();
                    }
                    Field::PrivateKey => {
                        self.private_key_content.pop();
                    }
                },

                KeyCode::Up if matches!(self.field, Field::PrivateKey) => {
                    self.field = Field::PublicKey
                }

                KeyCode::Down if matches!(self.field, Field::PublicKey) => {
                    self.field = Field::PrivateKey
                }

                _ => {}
            }
            None
        }

        fn render(&mut self, f: &mut ratatui::prelude::Frame) {
            let size = f.area();

            let block = Block::default()
                .title_bottom("Use ↑/↓ to move, enter to submit, esc to quit")
                .title_alignment(Alignment::Center);
            f.render_widget(block, size);

            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(70)])
                .split(size);

            let centered = Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    Constraint::Min(2),
                    Constraint::Percentage(70),
                    Constraint::Min(2),
                ])
                .split(chunks[1]);

            let logkeys_box = Block::bordered()
                .title(" Log your keys to acess your Blocktion account! ".bold())
                .title_alignment(Alignment::Center);
            f.render_widget(logkeys_box, centered[1]);

            let logkeys_chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Min(3),    // title
                    Constraint::Length(3), // pk
                    Constraint::Min(2),
                    Constraint::Length(3), // sk
                    Constraint::Min(3),
                ])
                .split(centered[1]);

            let input_box_layout = Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    Constraint::Min(2),
                    Constraint::Percentage(70),
                    Constraint::Min(2),
                ]);
            let mut input_box_pk = Paragraph::new(self.public_key_content.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("public key".bold())
                    .title_alignment(Alignment::Left),
            );

            let input_chunks_pk = input_box_layout.split(logkeys_chunks[1]);
            let input_chunks_sk = input_box_layout.split(logkeys_chunks[3]);

            let mut input_box_sk = Paragraph::new(self.private_key_content.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("secret key".bold())
                    .title_alignment(Alignment::Left),
            );

            match self.field {
                Field::PrivateKey => {
                    input_box_sk =
                        input_box_sk.style(Style::default().fg(ratatui::style::Color::LightYellow));
                }
                Field::PublicKey => {
                    input_box_pk =
                        input_box_pk.style(Style::default().fg(ratatui::style::Color::LightYellow));
                }
            };

            f.render_widget(input_box_pk, input_chunks_pk[1]);
            f.render_widget(input_box_sk, input_chunks_sk[1]);
        }
    }
}

mod bye {
    use crate::Screen;
    use ratatui::{
        layout::{Alignment, Constraint, Layout},
        widgets::Paragraph,
    };
    use std::time::Duration;

    pub const BYE_TIME: Duration = Duration::from_secs(1);

    pub struct Bye;

    impl Screen for Bye {
        fn handle_input(&mut self, _key: crossterm::event::KeyCode) -> Option<Box<dyn Screen>> {
            None
        }

        fn render(&mut self, f: &mut ratatui::prelude::Frame) {
            let [_, l, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .areas(f.area());
            f.render_widget(
                Paragraph::new("See you space cowboy...")
                    .centered()
                    .alignment(Alignment::Center),
                l,
            );
        }
    }
}

struct App {
    current_screen: Box<dyn Screen>,
}

impl App {
    fn new() -> Self {
        Self {
            current_screen: Box::new(LogKeys::new()),
        }
    }

    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.current_screen.render(frame))?;

            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Esc {
                    terminal.draw(|frame| Bye.render(frame))?;
                    sleep(BYE_TIME);

                    ratatui::restore();

                    return Ok(());
                }

                self.current_screen.handle_input(key.code);
            }
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let mut app = App::new();
    let result = app.run(terminal);
    result
}
