// use std::env;

use color_eyre::Result;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, SetCursorStyle},
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent,
        MouseEventKind,
    },
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
    scroll_offset: u16,
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
            scroll_offset: 0,
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
        let text_lines: Vec<Line> = self
            .messages
            .iter()
            .map(|msg| Line::from(Span::raw(msg)))
            .collect();

        // Calculate how many lines can fit inside the paragraph area
        let inner_height = area.height.saturating_sub(2);
        let total_lines = text_lines.len() as u16;

        // Ensure scroll_offset doesn't exceed what we have
        let max_scroll = total_lines.saturating_sub(inner_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        let messages_paragraph = Paragraph::new(Text::from(text_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Messages ")
                    .border_style(ratatui::style::Style::default())
                    .border_set(border::THICK),
            )
            .wrap(Wrap { trim: true })
            .scroll((scroll_offset, 0));

        // Create a layout for the messages area that includes a narrow column for the scrollbar
        let message_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        frame.render_widget(messages_paragraph, message_chunks[0]);

        // Draw a vertical scrollbar in message_chunks[1] if needed
        if total_lines > inner_height {
            let scrollbar_inner_height =
                message_chunks[1].height.saturating_sub(2);
            let thumb_pos = if max_scroll == 0 {
                0
            } else {
                scroll_offset * scrollbar_inner_height / max_scroll
            };

            let mut scrollbar_text = Vec::new();
            for i in 0..scrollbar_inner_height {
                if i == thumb_pos {
                    scrollbar_text.push(Line::from("█"));
                } else {
                    scrollbar_text.push(Line::from(" "));
                }
            }

            let scrollbar_paragraph =
                Paragraph::new(Text::from(scrollbar_text)).block(
                    Block::default()
                        .borders(Borders::LEFT | Borders::RIGHT)
                        .border_set(border::THICK),
                );

            frame.render_widget(scrollbar_paragraph, message_chunks[1]);
        } else {
            // If no scrolling needed, just draw borders
            let empty_scrollbar = Paragraph::new("").block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_set(border::THICK),
            );
            frame.render_widget(empty_scrollbar, message_chunks[1]);
        }
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

        // Set the cursor position.
        // In Insert mode, place it at the end of the input.
        // In Normal mode, place it at the start of the input line.
        let cursor_x = input_chunks[0].x + 1 + self.input.len() as u16;
        let cursor_y = input_chunks[0].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            Event::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
            }
            _ => {}
        }

        // TODO: Handle incoming messages from server:
        // while let Ok(msg) = self.rx.try_recv() {
        //     self.messages.push(msg);
        //     self.scroll_to_bottom();
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
            KeyCode::Char('j') => {
                self.scroll_down(1);
            }
            KeyCode::Char('k') => {
                self.scroll_up(1);
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

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(1);
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(1);
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
        self.scroll_to_bottom();
    }

    fn scroll_to_bottom(&mut self) {
        let total_lines = self.messages.len() as u16;
        self.scroll_offset = total_lines;
    }

    fn scroll_up(&mut self, lines: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    fn scroll_down(&mut self, lines: u16) {
        let total_lines = self.messages.len() as u16;
        self.scroll_offset = (self.scroll_offset + lines).min(total_lines);
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
