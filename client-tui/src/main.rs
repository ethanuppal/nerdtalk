// use std::env;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

#[tokio::main]
async fn main() -> Result<()> {
    // let url = env::args().nth(1).unwrap_or_else(|| {
    //     panic!("Pass the server's wss:// address as a command-line argument")
    // });

    let mut terminal = ratatui::init();
    let mut app = App::default();

    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug)]
pub struct App {
    messages: Vec<String>,
    input: String,
    exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            messages: vec![
                "Welcome to the chat!".into(),
                "Type a message and press Enter.".into()
            ],
            input: String::new(),
            exit: false,
        }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        // TODO: Use tokio::select! to poll for both user input and network messages. (rn there's no network)
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let size = frame.area();

        // Layout:
        // ┌───────────────────────┐
        // │       MESSAGES         │
        // │       (scrollable)     │
        // ├───────────────────────┤
        // │         INPUT          │
        // └───────────────────────┘
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(size);

        self.draw_messages_area(frame, chunks[0]);
        self.draw_input_area(frame, chunks[1]);
    }

    fn draw_messages_area(&self, frame: &mut Frame, area: Rect) {
        let text = self
            .messages
            .iter()
            .map(|msg| Line::from(Span::raw(msg)))
            .collect::<Vec<Line>>();

        let messages_paragraph = Paragraph::new(Text::from(text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Messages ")
                    .border_style(ratatui::style::Style::default())
                    .border_set(border::THICK)
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(messages_paragraph, area);
    }

    fn draw_input_area(&self, frame: &mut Frame, area: Rect) {
        let input_paragraph = Paragraph::new(Text::from(Span::raw(&self.input)))
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .wrap(Wrap { trim: false });
        frame.render_widget(input_paragraph, area);
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            _ => {}
        }

        // TODO: Handle incoming messages from the server.
        // For example:
        // while let Ok(msg) = self.rx.try_recv() {
        //     self.messages.push(msg);
        // }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Enter => self.send_message(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }

            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn send_message(&mut self) {
        let trimmed = self.input.trim();
        if !trimmed.is_empty() {
            self.messages.push(trimmed.to_string());
        }
        self.input.clear();
    }
}
