//use std::env;
use color_eyre::Result;
use copypasta::ClipboardContext;
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

mod vim;
use vim::{Mode, VimCmd};

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

pub struct App {
    messages: Vec<String>,
    input: String,
    mode: Mode,
    exit: bool,
    scroll_offset: u16,
    cursor_pos: usize,
    normal_mode_buffer: String,
    clipboard: ClipboardContext,
    undo_stack: Vec<String>,
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
            cursor_pos: 0,
            normal_mode_buffer: String::new(),
            clipboard: ClipboardContext::new().unwrap_or_else(|_| {
                eprintln!("Failed to initialize clipboard context.");
                copypasta::ClipboardContext::new().unwrap()
            }),
            undo_stack: Vec::new(),
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

    fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        let available_width_for_text = if size.width > 5 {
            size.width - 5
        } else {
            1 // fallback if terminal is very narrow
        };

        let line_count = if self.input.is_empty() {
            1
        } else {
            (self.input.len() as u16 + available_width_for_text - 1)
                / available_width_for_text
        };

        // We add 2 for the borders. line_count is the number of wrapped lines.
        let required_height = line_count + 2;
        // Ensure at least height 3
        let input_height = required_height.max(3);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(input_height)])
            .split(size);

        self.draw_messages_area(frame, chunks[0]);
        self.draw_input_area(frame, chunks[1], available_width_for_text);
    }

    fn draw_messages_area(&self, frame: &mut Frame, area: Rect) {
        let text_lines: Vec<Line> = self
            .messages
            .iter()
            .map(|msg| Line::from(Span::raw(msg)))
            .collect();

        let inner_height = area.height.saturating_sub(2);
        let total_lines = text_lines.len() as u16;

        let max_scroll = total_lines.saturating_sub(inner_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        let messages_paragraph = Paragraph::new(Text::from(text_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Messages ")
                    .border_set(border::THICK),
            )
            .wrap(Wrap { trim: true })
            .scroll((scroll_offset, 0));

        let message_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        frame.render_widget(messages_paragraph, message_chunks[0]);

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
            let empty_scrollbar = Paragraph::new("").block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_set(border::THICK),
            );
            frame.render_widget(empty_scrollbar, message_chunks[1]);
        }
    }

    fn draw_input_area(
        &self,
        frame: &mut Frame,
        area: Rect,
        available_width_for_text: u16,
    ) {
        let input_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3), // space for mode indicator
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

        // Compute where the cursor should go in TUI coordinates
        let line_index = if available_width_for_text > 0 {
            self.cursor_pos as u16 / available_width_for_text
        } else {
            0
        };
        let col_index = if available_width_for_text > 0 {
            self.cursor_pos as u16 % available_width_for_text
        } else {
            0
        };

        let cursor_x = input_chunks[0].x + 1 + col_index;
        let cursor_y = input_chunks[0].y + 1 + line_index;

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

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_key_event_normal_mode(key_event),
            Mode::Insert => self.handle_key_event_insert_mode(key_event),
        }
    }

    /// Normal mode: feed keys into VimCmd parser. Supports operator-pending commands like 'd w'.
    fn handle_key_event_normal_mode(&mut self, key_event: KeyEvent) {
        let ch = match key_event.code {
            KeyCode::Char('q') => {
                self.exit();
                return;
            }
            KeyCode::Char(c) => c,
            _ => {
                return;
            }
        };

        // 1) Add the pressed char to our normal-mode buffer
        self.normal_mode_buffer.push(ch);

        // 2) Parse the entire buffer with VimCmd
        let mut vim_cmd = VimCmd::new(&self.normal_mode_buffer);
        let commands = vim_cmd.parse();

        // 3) Apply the commands (if any)
        if !commands.is_empty() {
            vim_cmd.apply_cmds(
                &mut self.mode,
                &mut self.cursor_pos,
                &mut self.scroll_offset,
                self.messages.len() as u16,
                &mut self.input,
                &mut self.clipboard,
                &mut self.undo_stack,
                commands,
            );
        }

        // 4) Clear buffer if operator is no longer pending
        if !vim_cmd.is_operator_pending() {
            self.normal_mode_buffer.clear();
        }
    }

    /// Insert mode: typed characters are inserted into `self.input`.
    fn handle_key_event_insert_mode(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => self.send_message(),
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
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
        self.cursor_pos = 0;
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
