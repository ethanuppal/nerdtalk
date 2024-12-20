// use std::env;

use color_eyre::Result;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, SetCursorStyle},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::enable_raw_mode,
    ExecutableCommand,
};
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

    enable_raw_mode()?; // Ensure raw mode is enabled for cursor shape changes

    let mut terminal = ratatui::init();
    let mut app = App::default();

    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug)]
enum Mode {
    Insert,
    Normal,
}

#[derive(Debug)]
pub struct App {
    messages: Vec<String>,
    input: String,
    mode: Mode,
    exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            messages: vec![
                "Welcome to the chat!".into(),
                "Type a message and press Enter.".into(),
            ],
            input: String::new(),
            mode: Mode::Insert,
            exit: false,
        }
    }
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.update_cursor_shape(terminal)?;
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
        // │          INPUT         │
        // └───────────────────────┘
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
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
                    .border_set(border::THICK),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(messages_paragraph, area);
    }

    fn draw_input_area(&self, frame: &mut Frame, area: Rect) {
        // Divide the input area into two horizontal chunks:
        // Left: main input field
        // Right: mode indicator
        let input_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3), // space for indicator
            ])
            .split(area);

        let input_paragraph =
            Paragraph::new(Text::from(Span::raw(&self.input)))
                .block(Block::default().borders(Borders::ALL).title(" Input "))
                .wrap(Wrap { trim: false });

        let mode_str = match self.mode {
            Mode::Normal => Span::styled(
                "N",
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Blue),
            ),
            Mode::Insert => Span::styled(
                "I",
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Green),
            ),
        };

        let mode_paragraph = Paragraph::new(mode_str)
            .block(Block::default().borders(Borders::ALL));

        // Render widgets
        frame.render_widget(input_paragraph, input_chunks[0]);
        frame.render_widget(mode_paragraph, input_chunks[1]);

        let cursor_x = input_chunks[0].x + 1 + self.input.len() as u16;
        let cursor_y = input_chunks[0].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            _ => {}
        }

        // TODO: Handle incoming messages from the server.
        // while let Ok(msg) = self.rx.try_recv() {
        //     self.messages.push(msg);
        // }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_key_event_normal_mode(key_event),
            Mode::Insert => self.handle_key_event_insert_mode(key_event),
        }
    }

    fn handle_key_event_normal_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('i') | KeyCode::Char('a') => {
                self.mode = Mode::Insert;
            }
            _ => {}
        }
    }

    fn handle_key_event_insert_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
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

    fn update_cursor_shape(
        &self,
        terminal: &mut DefaultTerminal,
    ) -> Result<()> {
        match self.mode {
            Mode::Normal => {
                terminal.backend_mut().execute(EnableBlinking)?;
                terminal
                    .backend_mut()
                    .execute(SetCursorStyle::SteadyBlock)?;
            }
            Mode::Insert => {
                terminal.backend_mut().execute(DisableBlinking)?;
                terminal
                    .backend_mut()
                    .execute(SetCursorStyle::BlinkingBar)?;
            }
        }
        Ok(())
    }
}
