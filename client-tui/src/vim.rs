use std::collections::VecDeque;

use copypasta::ClipboardProvider;
use regex::Regex;

use crate::app::Focus;

/// Different Vim Modes
#[derive(Debug)]
pub enum Mode {
    Insert,
    Normal,
    Visual,
}

/// A [`Motion`] indicates movement over some text, e.g. `w`, `b`, `h`, etc.
#[derive(Clone, Copy, Debug)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    ForwardWord,
    ForwardBigWord,
    BackwardWord,
    BackwardBigWord,
    EndWord,
    StartFile,
    EndFile,
}

/// An [`Edit`] indicates a command which edits the text or sets up for an edit.
#[derive(Clone, Debug)]
pub enum Edit {
    Insert,
    Append,
    InsertSOL,
    AppendEOL,
    DeleteChar,
    DeleteEOL,
    Paste,
}

/// A Vim "Noun". This is the object following a verb (e.g. `dw`, `ciw`).
#[derive(Clone, Copy, Debug)]
pub enum Noun {
    Motion(Motion),
    InnerWord,
    InnerBigWord,
    Sentence,
    Parentheses,
    Braces,
    Angles,
    Apostrophes,
    Quotes,
    Backtick,
    Pending, // temporary state for initialization
}

/// Single-keystroke commands, e.g. `i`, `x`, `w`, etc.
#[derive(Clone, Debug)]
pub enum SingleCommand {
    Edit(Edit),
    Motion(Motion),
}

/// Our "operators" can embed a `Noun` (e.g., `delete(word)`).
#[derive(Clone, Debug)]
pub enum MultiCommand {
    Delete(Noun),
    Change(Noun),
    ChangeEOL,
    Replace(char),
    Yank(Noun),
}

/// A high-level [`Command`] is either:
///  - A single-key command (`SingleCommand`)
///  - Or an operator + object (`MultiCommand`).
#[derive(Clone, Debug)]
pub enum Command {
    SingleCommand(SingleCommand),
    MultiCommand(MultiCommand),
}

/// Simple [`VimCommand`] parser. Tracks an optional (pending) operator (e.g.
/// `d`, `c`, `y`).
pub struct CommandBuffer {
    chars: VecDeque<char>,
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self {
            chars: VecDeque::with_capacity(8),
        }
    }
}

impl CommandBuffer {
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn push(&mut self, input: char) {
        self.chars.push_back(input);
    }

    pub fn clear(&mut self) {
        self.chars.clear();
    }

    pub fn as_slice(&self) -> &[char] {
        self.chars.as_slices().0
    }

    pub fn parse(&mut self) -> Option<Command> {
        if self.is_empty() {
            return None;
        }

        self.chars.make_contiguous();

        if let Some(single_command) = self.parse_single_command() {
            self.advance(1);
            Some(Command::SingleCommand(single_command))
        } else if let Some((multi_command, length)) = self.parse_multi_command()
        {
            self.advance(length);
            Some(Command::MultiCommand(multi_command))
        } else {
            None
        }
    }

    fn parse_single_command(&mut self) -> Option<SingleCommand> {
        match self.current() {
            'i' => Some(SingleCommand::Edit(Edit::Insert)),
            'I' => Some(SingleCommand::Edit(Edit::InsertSOL)),
            'a' => Some(SingleCommand::Edit(Edit::Append)),
            'A' => Some(SingleCommand::Edit(Edit::AppendEOL)),
            'x' => Some(SingleCommand::Edit(Edit::DeleteChar)),
            'D' => Some(SingleCommand::Edit(Edit::DeleteEOL)),
            'p' => Some(SingleCommand::Edit(Edit::Paste)),
            'h' => Some(SingleCommand::Motion(Motion::Left)),
            'l' => Some(SingleCommand::Motion(Motion::Right)),
            'j' => Some(SingleCommand::Motion(Motion::Down)),
            'k' => Some(SingleCommand::Motion(Motion::Up)),
            'w' => Some(SingleCommand::Motion(Motion::ForwardWord)),
            'W' => Some(SingleCommand::Motion(Motion::ForwardBigWord)),
            'b' => Some(SingleCommand::Motion(Motion::BackwardWord)),
            'B' => Some(SingleCommand::Motion(Motion::BackwardBigWord)),
            'e' => Some(SingleCommand::Motion(Motion::EndWord)),
            '0' => Some(SingleCommand::Motion(Motion::StartFile)),
            '$' => Some(SingleCommand::Motion(Motion::EndFile)),
            _ => None,
        }
    }

    fn parse_multi_command(&mut self) -> Option<(MultiCommand, usize)> {
        match self.current() {
            'C' => Some((MultiCommand::ChangeEOL, 2)),
            'r' | 'R' => Some((MultiCommand::Replace(self.peek(1)?), 2)),
            _ => self.parse_multi_command_with_noun(),
        }
    }

    fn parse_multi_command_with_noun(
        &mut self,
    ) -> Option<(MultiCommand, usize)> {
        let (noun, noun_length) = self.peek_noun()?;
        match self.current() {
            'd' => Some((MultiCommand::Delete(noun), noun_length + 1)),
            'c' => Some((MultiCommand::Change(noun), noun_length + 1)),
            'y' => Some((MultiCommand::Yank(noun), noun_length + 1)),
            _ => None,
        }
    }

    fn peek_noun(&self) -> Option<(Noun, usize)> {
        match self.peek(1)? {
            'w' => Some((Noun::Motion(Motion::ForwardWord), 1)),
            'W' => Some((Noun::Motion(Motion::ForwardBigWord), 1)),
            'b' => Some((Noun::Motion(Motion::BackwardWord), 1)),
            'B' => Some((Noun::Motion(Motion::BackwardBigWord), 1)),
            'h' => Some((Noun::Motion(Motion::Left), 1)),
            'l' => Some((Noun::Motion(Motion::Right), 1)),
            'j' => Some((Noun::Motion(Motion::Down), 1)),
            'k' => Some((Noun::Motion(Motion::Up), 1)),

            // Possibly "iw", "iW", etc.
            'i' => match self.peek(2)? {
                'w' => Some((Noun::InnerWord, 2)),
                'W' => Some((Noun::InnerBigWord, 2)),
                's' => Some((Noun::Sentence, 2)),
                '(' | ')' => Some((Noun::Parentheses, 2)),
                '[' | ']' => Some((Noun::Braces, 2)),
                '{' | '}' => Some((Noun::Braces, 2)),
                '<' | '>' => Some((Noun::Angles, 2)),
                '\'' => Some((Noun::Apostrophes, 2)),
                '"' => Some((Noun::Quotes, 2)),
                '`' => Some((Noun::Backtick, 2)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn current(&self) -> char {
        self.peek(0).unwrap()
    }

    /// The `ahead`th zero-indexed character from the current position.
    /// `self.current()` is equivalent to `self.peek(0).unwrap()`.
    pub fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(ahead).cloned()
    }

    fn advance(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.chars.pop_front();
        }
    }
}

/// Vim without the text or rendering.
pub struct EditingContext {
    pub mode: Mode,
    pub focus: Focus,
    pub cursor_pos: usize,
    pub scroll_offset: u16,
    _undo_stack: Vec<String>,
}

impl Default for EditingContext {
    fn default() -> Self {
        Self {
            mode: Mode::Insert,
            focus: Focus::Input,
            scroll_offset: 0,
            cursor_pos: 0,
            _undo_stack: Vec::new(),
        }
    }
}

impl EditingContext {
    /// Applies a [`Command`]s to `text` (rendered at a window height of
    /// `height`) the current editor state, using `clipboard` for yanking
    /// and pasting
    pub fn apply_command(
        &mut self,
        text: &mut String,
        clipboard: &mut copypasta::ClipboardContext,
        height: u16,
        command: Command,
    ) {
        match command {
            // -----------------------------
            // SingleCommand actions
            // -----------------------------
            Command::SingleCommand(single_cmd) => match single_cmd {
                SingleCommand::Edit(edit) => {
                    if self.focus == Focus::Input {
                        match edit {
                            Edit::Insert => {
                                self.mode = Mode::Insert;
                            }
                            Edit::Append => {
                                self.mode = Mode::Insert;
                                if self.cursor_pos < text.len() {
                                    self.cursor_pos += 1;
                                }
                            }
                            Edit::InsertSOL => {
                                self.mode = Mode::Insert;
                                self.cursor_pos = 0;
                            }
                            Edit::AppendEOL => {
                                self.mode = Mode::Insert;
                                self.cursor_pos = text.len();
                            }
                            Edit::DeleteChar => {
                                if self.cursor_pos < text.len() {
                                    let removed_char =
                                        text.remove(self.cursor_pos);
                                    let _ = clipboard
                                        .set_contents(removed_char.to_string());
                                }
                            }
                            Edit::DeleteEOL => {
                                if self.cursor_pos < text.len() {
                                    let removed = text
                                        .drain(self.cursor_pos..)
                                        .collect::<String>();
                                    let _ = clipboard.set_contents(removed);
                                }
                            }
                            Edit::Paste => {
                                if let Ok(clip_text) = clipboard.get_contents()
                                {
                                    text.insert_str(
                                        self.cursor_pos,
                                        &clip_text,
                                    );
                                    self.cursor_pos += clip_text.len();
                                }
                            }
                        }
                    }
                }

                SingleCommand::Motion(motion) => {
                    match motion {
                        Motion::Left => {
                            if self.focus == Focus::Messages {
                                // Not implemented. Could do horizontal scroll in
                                // messages
                            } else if self.cursor_pos > 0 {
                                self.cursor_pos -= 1;
                            }
                        }
                        Motion::Right => {
                            if self.focus == Focus::Messages {
                                // Not implemented
                            } else if self.cursor_pos < text.len() {
                                self.cursor_pos += 1;
                            }
                        }
                        Motion::Up => {
                            if self.focus == Focus::Messages {
                                self.scroll_offset =
                                    self.scroll_offset.saturating_sub(1);
                            }
                        }
                        Motion::Down => {
                            if self.focus == Focus::Messages {
                                self.scroll_offset =
                                    (self.scroll_offset + 1).min(height);
                            }
                        }
                        Motion::ForwardWord => {
                            self.cursor_pos =
                                find_next_word_boundary(text, self.cursor_pos);
                        }
                        Motion::ForwardBigWord => {
                            self.cursor_pos = find_next_big_word_boundary(
                                text,
                                self.cursor_pos,
                            );
                        }
                        Motion::BackwardWord => {
                            self.cursor_pos =
                                find_prev_word_boundary(text, self.cursor_pos);
                        }
                        Motion::BackwardBigWord => {
                            self.cursor_pos = find_prev_big_word_boundary(
                                text,
                                self.cursor_pos,
                            );
                        }
                        Motion::EndWord => {
                            self.cursor_pos =
                                find_word_end(text, self.cursor_pos);
                        }

                        Motion::StartFile => {
                            if self.focus == Focus::Messages {
                                self.scroll_offset = 0;
                            } else {
                                self.cursor_pos = 0;
                            }
                        }
                        Motion::EndFile => {
                            if self.focus == Focus::Messages {
                                self.scroll_offset = height;
                            } else {
                                self.cursor_pos = text.len();
                            }
                        }
                    }
                }
            },
            // -----------------------------
            // MultiCommand actions
            // -----------------------------
            Command::MultiCommand(multi_cmd) => {
                if self.focus == Focus::Input {
                    // Normal input editing
                    match multi_cmd {
                        MultiCommand::Delete(noun) => {
                            delete_helper(
                                &mut self.cursor_pos,
                                text,
                                clipboard,
                                noun,
                            );
                        }
                        MultiCommand::Change(noun) => {
                            change_helper(
                                &mut self.mode,
                                &mut self.cursor_pos,
                                text,
                                clipboard,
                                noun,
                            );
                        }
                        MultiCommand::Yank(noun) => {
                            yank_helper(
                                &mut self.cursor_pos,
                                text,
                                clipboard,
                                noun,
                            );
                        }
                        MultiCommand::ChangeEOL => {
                            if self.cursor_pos < text.len() {
                                let removed = text
                                    .drain(self.cursor_pos..)
                                    .collect::<String>();
                                let _ = clipboard.set_contents(removed);
                            }
                            self.mode = Mode::Insert;
                        }
                        MultiCommand::Replace(c) => {
                            if self.cursor_pos < text.len() {
                                text.remove(self.cursor_pos);
                                text.insert(self.cursor_pos, c);
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------
//  Helpers for deleting, changing, yanking
// ------------------------------------------
fn delete_helper(
    cursor_pos: &mut usize,
    text: &mut String,
    clipboard: &mut copypasta::ClipboardContext,
    noun: Noun,
) {
    match noun {
        Noun::Motion(Motion::ForwardWord) => {
            let end_pos = find_next_word_boundary(text, *cursor_pos);
            if end_pos > *cursor_pos {
                let removed =
                    text.drain(*cursor_pos..end_pos).collect::<String>();
                let _ = clipboard.set_contents(removed);
            }
        }
        Noun::Motion(Motion::ForwardBigWord) => {
            let end_pos = find_next_big_word_boundary(text, *cursor_pos);
            if end_pos > *cursor_pos {
                let removed =
                    text.drain(*cursor_pos..end_pos).collect::<String>();
                let _ = clipboard.set_contents(removed);
            }
        }
        Noun::Motion(Motion::BackwardWord) => {
            let start_pos = find_prev_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos {
                let removed =
                    text.drain(start_pos..*cursor_pos).collect::<String>();
                let _ = clipboard.set_contents(removed);
                *cursor_pos = start_pos;
            }
        }
        Noun::Motion(Motion::BackwardBigWord) => {
            let start_pos = find_prev_big_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos {
                let removed =
                    text.drain(start_pos..*cursor_pos).collect::<String>();
                let _ = clipboard.set_contents(removed);
                *cursor_pos = start_pos;
            }
        }
        Noun::InnerWord => {
            let start_pos = find_prev_word_boundary(text, *cursor_pos);
            let end_pos = find_next_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos && end_pos > *cursor_pos {
                let removed =
                    text.drain(start_pos..end_pos).collect::<String>();
                let _ = clipboard.set_contents(removed);
                *cursor_pos = start_pos;
            }
        }
        Noun::InnerBigWord => {
            let start_pos = find_prev_big_word_boundary(text, *cursor_pos);
            let end_pos = find_next_big_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos && end_pos > *cursor_pos {
                let removed =
                    text.drain(start_pos..end_pos).collect::<String>();
                let _ = clipboard.set_contents(removed);
                *cursor_pos = start_pos;
            }
        }
        _ => {}
    }
}

fn change_helper(
    mode: &mut Mode,
    cursor_pos: &mut usize,
    text: &mut String,
    clipboard: &mut copypasta::ClipboardContext,
    noun: Noun,
) {
    delete_helper(cursor_pos, text, clipboard, noun);
    *mode = Mode::Insert;
}

fn yank_helper(
    cursor_pos: &mut usize,
    text: &str,
    clipboard: &mut copypasta::ClipboardContext,
    noun: Noun,
) {
    match noun {
        Noun::Motion(Motion::ForwardWord) => {
            let end_pos = find_next_word_boundary(text, *cursor_pos);
            if end_pos > *cursor_pos {
                let selection = &text[*cursor_pos..end_pos];
                let _ = clipboard.set_contents(selection.to_string());
            }
        }
        Noun::Motion(Motion::ForwardBigWord) => {
            let end_pos = find_next_big_word_boundary(text, *cursor_pos);
            if end_pos > *cursor_pos {
                let selection = &text[*cursor_pos..end_pos];
                let _ = clipboard.set_contents(selection.to_string());
            }
        }
        Noun::Motion(Motion::BackwardWord) => {
            let start_pos = find_prev_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos {
                let selection = &text[start_pos..*cursor_pos];
                let _ = clipboard.set_contents(selection.to_string());
            }
        }
        Noun::Motion(Motion::BackwardBigWord) => {
            let start_pos = find_prev_big_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos {
                let selection = &text[start_pos..*cursor_pos];
                let _ = clipboard.set_contents(selection.to_string());
            }
        }
        Noun::InnerWord => {
            let start_pos = find_prev_word_boundary(text, *cursor_pos);
            let end_pos = find_next_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos && end_pos > *cursor_pos {
                let selection = &text[start_pos..end_pos];
                let _ = clipboard.set_contents(selection.to_string());
            }
        }
        Noun::InnerBigWord => {
            let start_pos = find_prev_big_word_boundary(text, *cursor_pos);
            let end_pos = find_next_big_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos && end_pos > *cursor_pos {
                let selection = &text[start_pos..end_pos];
                let _ = clipboard.set_contents(selection.to_string());
            }
        }
        _ => {}
    }
}

// --------------------- Word boundary helpers -----------------------
fn word_boundary(
    text: &str,
    start_index: usize,
    boundary_regex: &str,
    is_forward: bool,
) -> usize {
    let remainder = if is_forward {
        &text[start_index..]
    } else {
        &text[..start_index]
    };

    let regex = Regex::new(boundary_regex).unwrap();
    let matches: Vec<_> = regex.find_iter(remainder).collect();

    if is_forward {
        if let Some(mat) = matches.first() {
            let mut ms = mat.start();
            if mat.as_str().chars().all(char::is_whitespace) {
                ms = mat.end();
            }
            start_index + ms + (ms == 0) as usize
        } else {
            text.len()
        }
    } else {
        if let Some(mat) = matches.last() {
            let (ms, me) = if mat.end() == start_index {
                let (new_index, at_front) = matches.len().overflowing_sub(2);
                if at_front {
                    (0, 0)
                } else {
                    (matches[new_index].start(), matches[new_index].end())
                }
            } else {
                (mat.start(), mat.end())
            };
            me + (text[ms..me + 1].chars().all(char::is_whitespace) as usize)
        } else {
            0
        }
    }
}

fn find_next_word_boundary(text: &str, start_index: usize) -> usize {
    word_boundary(text, start_index, r"[\s]+|[\p{P}]", true)
}

fn find_next_big_word_boundary(text: &str, start_index: usize) -> usize {
    word_boundary(text, start_index, r"[\s]+", true)
}

fn find_prev_word_boundary(text: &str, start_index: usize) -> usize {
    word_boundary(text, start_index, r"[\s]+|[\p{P}]", false)
}

fn find_prev_big_word_boundary(text: &str, start_index: usize) -> usize {
    word_boundary(text, start_index, r"[\s]+", false)
}

fn find_word_end(text: &str, start_index: usize) -> usize {
    let remainder = &text[start_index..];
    let regex = Regex::new(r"[\s]+|[\p{P}]").unwrap();
    let matches: Vec<_> = regex.find_iter(remainder).collect();

    if let Some(mat) = matches.first() {
        let mut ms = mat.start();
        if mat.as_str().chars().all(char::is_whitespace) {
            ms = mat.end();
        }
        if start_index + ms - 2 == start_index {
            // let (new_index, at_end) = (1 as usize).overflowing_add(2);

            start_index + matches.get(1).unwrap_or(mat).end() - 2
        } else {
            start_index + ms - 2
        }
    } else {
        text.len()
    }
}
