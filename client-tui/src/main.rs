use std::{env, io, sync::Arc, time};

use copypasta::ClipboardContext;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, SetCursorStyle},
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent,
        MouseEventKind,
    },
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

mod vim;
use tokio::{
    sync::{mpsc, RwLock},
    time::Instant,
};
use vim::{Mode, VimCommand};

/// Indicates which part of the UI is currently in “focus.”
#[derive(Debug, PartialEq)]
pub enum Focus {
    Messages,
    Input,
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let url = env::args().nth(1).unwrap_or_else(|| {
        panic!("Pass the server's wss:// address as a command-line argument")
    });

    let (connection, tx, mut rx) = client_connect::connect_to_server(&url)
        .await
        .map_err(io::Error::other)?;

    let messages = Arc::new(RwLock::new(vec![
        "Welcome to the chat!".into(),
        "Type a message and press Enter.".into(),
    ]));
    let app_messages = messages.clone();

    let mut app = App::new(tx);

    tokio::spawn(async move {
        while let Some(server_message) = rx.recv().await {
            let server_message = server_message.expect("todo");
            match server_message {
                comms::ServerMessage::NewEntry(chat_log_entry) => {
                    messages.write().await.push(format!(
                        "{}: {}",
                        chat_log_entry.username, chat_log_entry.content
                    ));
                }
            }
        }
    });

    crossterm::terminal::enable_raw_mode()?; // Ensure raw mode is enabled for cursor shape changes
    let mut terminal = ratatui::init();

    let app_result = app.run(&mut terminal, app_messages).await;
    ratatui::restore();
    connection.close();
    app_result
}

pub struct App {
    input: String,
    mode: Mode,
    exit: bool,
    scroll_offset: u16,
    messages_cursor: usize,
    cursor_pos: usize,
    normal_mode_buffer: String,
    clipboard: ClipboardContext,
    undo_stack: Vec<String>,
    focus: Focus,
    tx: mpsc::UnboundedSender<comms::ClientMessage>,
}

impl App {
    pub fn new(tx: mpsc::UnboundedSender<comms::ClientMessage>) -> Self {
        Self {
            input: String::new(),
            mode: Mode::Insert,
            exit: false,
            scroll_offset: 0,
            messages_cursor: 0,
            cursor_pos: 0,
            normal_mode_buffer: String::new(),
            clipboard: ClipboardContext::new().unwrap_or_else(|_| {
                eprintln!("Failed to initialize clipboard context.");
                copypasta::ClipboardContext::new().unwrap()
            }),
            undo_stack: Vec::new(),
            focus: Focus::Input,
            tx,
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        messages: Arc<RwLock<Vec<String>>>,
    ) -> Result<(), io::Error> {
        let mut interval =
            tokio::time::interval(time::Duration::from_millis(20));
        while !self.exit {
            {
                let messages = messages.read().await;
                let messages_ref = &*messages;
                terminal.draw(|frame| self.draw(messages_ref, frame))?;
                self.update_cursor_shape(terminal)?;
                self.handle_events(messages_ref)?;
                drop(messages);
            }
            interval.tick().await;
        }
        Ok(())
    }

    fn draw(&mut self, messages: &[String], frame: &mut Frame) {
        let size = frame.area();

        let available_width_for_text = if size.width > 5 {
            size.width - 5
        } else {
            1 // fallback if terminal is very narrow
        };

        let line_count = if self.input.is_empty() {
            1
        } else {
            (self.input.len() as u16).div_ceil(available_width_for_text)
        };

        // We add 2 for the borders. line_count is the number of wrapped lines.
        let required_height = line_count + 2;
        // Ensure at least height 3 for the input box
        let input_height = required_height.max(3);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(input_height)])
            .split(size);

        self.draw_messages_area(messages, frame, chunks[0]);
        self.draw_input_area(frame, chunks[1], available_width_for_text);
    }

    fn draw_messages_area(
        &mut self,
        messages: &[String],
        frame: &mut Frame,
        area: Rect,
    ) {
        let text_lines: Vec<Line> = messages
            .iter()
            .map(|msg| Line::from(Span::raw(msg)))
            .collect();

        let inner_height = area.height.saturating_sub(2);
        let total_lines = text_lines.len() as u16;

        if self.messages_cursor >= messages.len() {
            self.messages_cursor = messages.len().saturating_sub(1);
        }

        let max_scroll = total_lines.saturating_sub(inner_height);
        self.scroll_offset = self.scroll_offset.min(max_scroll);

        if (self.messages_cursor as u16) < self.scroll_offset {
            self.scroll_offset = self.messages_cursor as u16;
        } else if (self.messages_cursor as u16)
            >= (self.scroll_offset + inner_height)
        {
            self.scroll_offset =
                (self.messages_cursor as u16).saturating_sub(inner_height - 1);
        }

        let messages_paragraph = Paragraph::new(Text::from(text_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Messages ")
                    .border_set(border::THICK),
            )
            .wrap(Wrap { trim: true })
            .scroll((self.scroll_offset, 0));

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
                self.scroll_offset * scrollbar_inner_height / max_scroll
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
            // no scrollbar needed
            let empty_scrollbar = Paragraph::new("").block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_set(border::THICK),
            );
            frame.render_widget(empty_scrollbar, message_chunks[1]);
        }

        if self.focus == Focus::Messages {
            let relative_y = (self.messages_cursor as u16)
                .saturating_sub(self.scroll_offset);
            let cursor_x = message_chunks[0].x + 1;
            let cursor_y = message_chunks[0].y + 1 + relative_y;
            frame.set_cursor_position((cursor_x, cursor_y));
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
            .constraints([Constraint::Min(1), Constraint::Length(5)])
            .split(area);

        let input_paragraph =
            Paragraph::new(Text::from(Span::raw(&self.input)))
                .block(Block::default().borders(Borders::ALL).title(" Input "))
                .wrap(Wrap { trim: false });

        let mode_span = self.mode_indicator_span();
        let mode_paragraph = Paragraph::new(mode_span)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(input_paragraph, input_chunks[0]);
        frame.render_widget(mode_paragraph, input_chunks[1]);

        if self.focus == Focus::Input {
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
    }

    fn handle_events(&mut self, messages: &[String]) -> Result<(), io::Error> {
        if event::poll(time::Duration::from_millis(5))? {
            match event::read()? {
                Event::Key(key_event)
                    if key_event.kind == KeyEventKind::Press =>
                {
                    self.handle_key_event(messages, key_event);
                }
                Event::Mouse(mouse_event) => {
                    self.handle_mouse_event(messages, mouse_event);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, messages: &[String], key_event: KeyEvent) {
        match self.mode {
            Mode::Normal => {
                self.handle_key_event_normal_mode(messages, key_event)
            }
            Mode::Insert => {
                self.handle_key_event_insert_mode(messages, key_event)
            }
        }
    }

    /// Normal mode: typed characters are interpreted as Vim commands.
    fn handle_key_event_normal_mode(
        &mut self,
        messages: &[String],
        key_event: KeyEvent,
    ) {
        match key_event.code {
            KeyCode::Char('j') => {
                if self.focus == Focus::Messages {
                    if self.messages_cursor + 1 < messages.len() {
                        self.messages_cursor += 1;
                    } else {
                        self.focus = Focus::Input;
                    }
                }
            }
            KeyCode::Char('k') => {
                if self.focus == Focus::Messages {
                    if self.messages_cursor > 0 {
                        self.messages_cursor -= 1;
                    }
                } else {
                    // safe assumption, there should always be a message
                    if !messages.is_empty() {
                        self.messages_cursor = messages.len() - 1;
                    }
                    self.focus = Focus::Messages;
                }
            }
            KeyCode::Char('q') => {
                self.exit();
                return;
            }
            // Use the Vim engine for the rest
            KeyCode::Char(c) => {
                self.normal_mode_buffer.push(c);
            }
            _ => {}
        };

        // Now parse the normal-mode buffer
        let mut vim_cmd = VimCommand::new(&self.normal_mode_buffer);
        let commands = vim_cmd.parse();
        if !commands.is_empty() {
            vim_cmd.apply_cmds(
                &mut self.mode,
                &mut self.focus,
                &mut self.cursor_pos,
                &mut self.scroll_offset,
                messages.len() as u16,
                &mut self.input,
                &mut self.clipboard,
                &mut self.undo_stack,
                commands,
            );
        }
        // Clear if we're no longer pending
        if !vim_cmd.is_operator_pending() {
            self.normal_mode_buffer.clear();
        }
    }

    /// Insert mode: typed characters are inserted into `self.input`.
    fn handle_key_event_insert_mode(
        &mut self,
        messages: &[String],
        key_event: KeyEvent,
    ) {
        match key_event.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                self.cursor_pos = (self.cursor_pos + 1).min(self.input.len());
            }
            KeyCode::Enter => self.send_message(messages),
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

    fn handle_mouse_event(
        &mut self,
        messages: &[String],
        mouse_event: MouseEvent,
    ) {
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(1);
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(messages, 1);
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn send_message(&mut self, messages: &[String]) {
        let trimmed = self.input.trim();
        if !trimmed.is_empty() {
            self.tx
                .send(comms::ClientMessage::Append(comms::AppendChatEntry {
                    username: "me".to_string(),
                    content: trimmed.to_string(),
                }))
                .expect("channel closed on server");
        }
        self.input.clear();
        self.cursor_pos = 0;
        self.scroll_to_bottom(messages);
    }

    fn scroll_to_bottom(&mut self, messages: &[String]) {
        if !messages.is_empty() {
            self.messages_cursor = messages.len() - 1;
        }
        let total_lines = messages.len() as u16;
        self.scroll_offset = total_lines.saturating_sub(1);
    }

    fn scroll_up(&mut self, lines: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        if self.messages_cursor as u16 >= self.scroll_offset {
            // keep the messages_cursor in sync if needed
        }
    }

    fn scroll_down(&mut self, messages: &[String], lines: u16) {
        let total_lines = messages.len() as u16;
        self.scroll_offset =
            (self.scroll_offset + lines).min(total_lines.saturating_sub(1));
    }

    fn update_cursor_shape(
        &self,
        terminal: &mut DefaultTerminal,
    ) -> Result<(), io::Error> {
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

    fn mode_indicator_span(&self) -> ratatui::text::Span {
        match self.mode {
            Mode::Normal => {
                if self.normal_mode_buffer.is_empty() {
                    Span::styled(
                        " N ".to_string(),
                        Style::default().fg(Color::Blue),
                    )
                } else {
                    let display_str = &self.normal_mode_buffer
                        [..self.normal_mode_buffer.len().min(4)];
                    let padded = format!("{:<4}", display_str);

                    Span::styled(
                        padded.to_string(),
                        Style::default().fg(Color::Blue),
                    )
                }
            }
            Mode::Insert => Span::styled(
                " I ".to_string(),
                Style::default().fg(Color::Green),
            ),
        }
    }
}
