use std::{io, sync::Arc, time};

use copypasta::{ClipboardContext, ClipboardProvider};
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
use tokio::sync::{mpsc, RwLock};

use crate::vim;

/// Indicates which part of the UI is currently in “focus.”
#[derive(Debug, PartialEq)]
pub enum Focus {
    Messages,
    Input,
}

pub struct App {
    input: String,
    editing_context: vim::EditingContext,
    exit: bool,
    messages_cursor: usize,
    command_buffer: vim::CommandBuffer,
    clipboard: ClipboardContext,
    tx: mpsc::UnboundedSender<comms::ClientMessage>,
    visual_anchor: Option<usize>,
}

impl App {
    pub fn new(tx: mpsc::UnboundedSender<comms::ClientMessage>) -> Self {
        Self {
            input: String::new(),
            editing_context: vim::EditingContext::default(),
            exit: false,
            messages_cursor: 0,
            command_buffer: vim::CommandBuffer::default(),
            clipboard: ClipboardContext::new().unwrap_or_else(|_| {
                eprintln!("Failed to initialize clipboard context.");
                copypasta::ClipboardContext::new().unwrap()
            }),
            tx,
            visual_anchor: None,
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
        self.editing_context.scroll_offset =
            self.editing_context.scroll_offset.min(max_scroll);

        if (self.messages_cursor as u16) < self.editing_context.scroll_offset {
            self.editing_context.scroll_offset = self.messages_cursor as u16;
        } else if (self.messages_cursor as u16)
            >= (self.editing_context.scroll_offset + inner_height)
        {
            self.editing_context.scroll_offset =
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
            .scroll((self.editing_context.scroll_offset, 0));

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
                self.editing_context.scroll_offset * scrollbar_inner_height
                    / max_scroll
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

        if self.editing_context.focus == Focus::Messages {
            let relative_y = (self.messages_cursor as u16)
                .saturating_sub(self.editing_context.scroll_offset);
            let relative_x = if self.editing_context.cursor_pos
                > messages[self.messages_cursor].len()
            {
                (messages[self.messages_cursor].len().saturating_sub(1)) as u16
            } else {
                self.editing_context.cursor_pos as u16
            };

            let cursor_x = message_chunks[0].x + 1 + relative_x;
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
        let displayed_text =
            if let vim::Mode::Visual = self.editing_context.mode {
                render_input_with_selection(
                    &self.input,
                    self.visual_anchor,
                    self.editing_context.cursor_pos,
                )
            } else {
                Text::from(Span::raw(&self.input))
            };

        let input_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(5)])
            .split(area);

        let input_paragraph = Paragraph::new(Text::from(displayed_text))
            .block(Block::default().borders(Borders::ALL).title(" Input "))
            .wrap(Wrap { trim: false });

        let mode_span = self.mode_indicator_span();
        let mode_paragraph = Paragraph::new(mode_span)
            .block(Block::default().borders(Borders::ALL));

        frame.render_widget(input_paragraph, input_chunks[0]);
        frame.render_widget(mode_paragraph, input_chunks[1]);

        if self.editing_context.focus == Focus::Input {
            let line_index = if available_width_for_text > 0 {
                self.editing_context.cursor_pos as u16
                    / available_width_for_text
            } else {
                0
            };
            let col_index = if available_width_for_text > 0 {
                self.editing_context.cursor_pos as u16
                    % available_width_for_text
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
        match self.editing_context.mode {
            vim::Mode::Normal => {
                self.handle_key_event_normal_mode(messages, key_event)
            }
            vim::Mode::Insert => {
                self.handle_key_event_insert_mode(messages, key_event)
            }
            vim::Mode::Visual => {
                self.handle_key_event_visual_mode(messages, key_event)
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
                if self.editing_context.focus == Focus::Messages {
                    if self.messages_cursor + 1 < messages.len() {
                        self.messages_cursor += 1;
                    } else {
                        self.editing_context.focus = Focus::Input;
                    }
                }
            }
            KeyCode::Char('k') => {
                if self.editing_context.focus == Focus::Messages {
                    if self.messages_cursor > 0 {
                        self.messages_cursor -= 1;
                    }
                } else {
                    // safe assumption, there should always be a message
                    if !messages.is_empty() {
                        self.messages_cursor = messages.len() - 1;
                    }
                    self.editing_context.focus = Focus::Messages;
                }
            }
            KeyCode::Char('q') => {
                self.exit();
                return;
            }
            KeyCode::Esc => {
                self.command_buffer.clear();
                return;
            }
            KeyCode::Char('v') => {
                self.editing_context.mode = vim::Mode::Visual;
                self.visual_anchor = Some(self.editing_context.cursor_pos);
                self.command_buffer.clear();
                return;
            }
            // Use the Vim engine for the rest
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            _ => {}
        };

        if let Some(command) = self.command_buffer.parse() {
            self.editing_context.apply_command(
                &mut self.input,
                &mut self.clipboard,
                messages.len() as u16,
                command,
            );
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
                self.editing_context.mode = vim::Mode::Normal;
            }
            KeyCode::Left => {
                self.editing_context.cursor_pos =
                    self.editing_context.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                self.editing_context.cursor_pos =
                    (self.editing_context.cursor_pos + 1).min(self.input.len());
            }
            KeyCode::Enter => self.send_message(messages),
            KeyCode::Backspace => {
                if self.editing_context.cursor_pos > 0 {
                    self.editing_context.cursor_pos -= 1;
                    self.input.remove(self.editing_context.cursor_pos);
                }
            }
            KeyCode::Char(c) => {
                self.input.insert(self.editing_context.cursor_pos, c);
                self.editing_context.cursor_pos += 1;
            }
            _ => {}
        }
    }

    fn handle_key_event_visual_mode(
        &mut self,
        messages: &[String],
        key_event: KeyEvent,
    ) {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('v') => {
                self.editing_context.mode = vim::Mode::Normal;
                self.visual_anchor = None;
            }
            KeyCode::Left => {
                self.editing_context.cursor_pos =
                    self.editing_context.cursor_pos.saturating_sub(1);
            }
            KeyCode::Right => {
                self.editing_context.cursor_pos =
                    (self.editing_context.cursor_pos + 1).min(self.input.len());
            }
            KeyCode::Char('h') => {
                self.editing_context.cursor_pos =
                    self.editing_context.cursor_pos.saturating_sub(1);
            }
            KeyCode::Char('l') => {
                self.editing_context.cursor_pos =
                    (self.editing_context.cursor_pos + 1).min(self.input.len());
            }

            KeyCode::Char('y') => {
                self.yank_visual_selection();
                self.editing_context.mode = vim::Mode::Normal;
                self.visual_anchor = None;
            }

            KeyCode::Char('d') => {
                self.delete_visual_selection();
                self.editing_context.mode = vim::Mode::Normal;
                self.visual_anchor = None;
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
        self.editing_context.cursor_pos = 0;
        self.scroll_to_bottom(messages);
    }

    fn scroll_to_bottom(&mut self, messages: &[String]) {
        if !messages.is_empty() {
            self.messages_cursor = messages.len() - 1;
        }
        let total_lines = messages.len() as u16;
        self.editing_context.scroll_offset = total_lines.saturating_sub(1);
    }

    fn scroll_up(&mut self, lines: u16) {
        self.editing_context.scroll_offset =
            self.editing_context.scroll_offset.saturating_sub(lines);
        if self.messages_cursor as u16 >= self.editing_context.scroll_offset {
            // keep the messages_cursor in sync if needed
        }
    }

    fn scroll_down(&mut self, messages: &[String], lines: u16) {
        let total_lines = messages.len() as u16;
        self.editing_context.scroll_offset =
            (self.editing_context.scroll_offset + lines)
                .min(total_lines.saturating_sub(1));
    }

    fn yank_visual_selection(&mut self) {
        if let Some(anchor) = self.visual_anchor {
            let (start, end) = if anchor <= self.editing_context.cursor_pos {
                (anchor, self.editing_context.cursor_pos)
            } else {
                (self.editing_context.cursor_pos, anchor)
            };
            if start < end && end <= self.input.len() {
                let selected = &self.input[start..end];
                let _ = self.clipboard.set_contents(selected.to_string());
            }
        }
    }

    fn delete_visual_selection(&mut self) {
        if let Some(anchor) = self.visual_anchor {
            let (start, end) = if anchor <= self.editing_context.cursor_pos {
                (anchor, self.editing_context.cursor_pos)
            } else {
                (self.editing_context.cursor_pos, anchor)
            };
            if start < end && end <= self.input.len() {
                let selected = &self.input[start..end];
                let _ = self.clipboard.set_contents(selected.to_string());

                self.input.drain(start..end);
                self.editing_context.cursor_pos = start;
            }
        }
    }

    fn update_cursor_shape(
        &self,
        terminal: &mut DefaultTerminal,
    ) -> Result<(), io::Error> {
        match self.editing_context.mode {
            vim::Mode::Normal => {
                terminal.backend_mut().execute(EnableBlinking)?;
                terminal
                    .backend_mut()
                    .execute(SetCursorStyle::SteadyBlock)?;
            }
            vim::Mode::Insert => {
                terminal.backend_mut().execute(DisableBlinking)?;
                terminal
                    .backend_mut()
                    .execute(SetCursorStyle::BlinkingBar)?;
            }
            vim::Mode::Visual => {
                terminal.backend_mut().execute(DisableBlinking)?;
                terminal
                    .backend_mut()
                    .execute(SetCursorStyle::SteadyUnderScore)?;
            }
        }
        Ok(())
    }

    fn mode_indicator_span(&self) -> ratatui::text::Span {
        match self.editing_context.mode {
            vim::Mode::Normal => {
                if self.command_buffer.is_empty() {
                    Span::styled(
                        " N ".to_string(),
                        Style::default().fg(Color::Blue),
                    )
                } else {
                    let mut display_str =
                        self.command_buffer.current().to_string();
                    if let Some(c) = self.command_buffer.peek(1) {
                        display_str.push(c);
                    }
                    let padded = format!("{:<4}", display_str);

                    Span::styled(padded, Style::default().fg(Color::Blue))
                }
            }
            vim::Mode::Insert => Span::styled(
                " I ".to_string(),
                Style::default().fg(Color::Green),
            ),
            vim::Mode::Visual => Span::styled(
                " V ".to_string(),
                Style::default().fg(Color::Magenta),
            ),
        }
    }
}

/// A small helper function to render the input text with a highlighted region
/// (for Visual mode) between visual_anchor and cursor_pos.
fn render_input_with_selection(
    text: &str,
    visual_anchor: Option<usize>,
    cursor_pos: usize,
) -> Text<'_> {
    if visual_anchor.is_none() {
        return Text::from(Span::raw(text));
    }
    let anchor = visual_anchor.unwrap();
    let (start, end) = if anchor <= cursor_pos {
        (anchor, cursor_pos)
    } else {
        (cursor_pos, anchor)
    };

    // Safety checks
    if start >= end || end > text.len() {
        return Text::from(Span::raw(text));
    }

    let before = &text[..start];
    let selected = &text[start..end];
    let after = &text[end..];

    Text::from(Line::from(vec![
        Span::raw(before),
        Span::styled(
            selected,
            Style::default().bg(Color::LightBlue).fg(Color::Black),
        ),
        Span::raw(after),
    ]))
}
