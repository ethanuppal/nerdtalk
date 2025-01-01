use std::collections::VecDeque;

use copypasta::ClipboardProvider;
use regex::Regex;

// We import the new Focus enum so we can block editing if focus ==
// Messages
use crate::app::Focus;

/// Different Vim Modes
#[derive(Debug)]
pub enum Mode {
    Insert,
    Normal,
}

/// Single-keystroke commands, e.g. `i`, `x`, `w`, etc.
#[derive(Clone, Debug)]
pub enum SingleCommand {
    Insert,
    Append,
    InsertSOL,
    AppendEOL,
    DeleteCharUnderCursor,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    ForwardWord,
    ForwardBigWord,
    BackwardWord,
    BackwardBigWord,
    Paste,
    StartFile,
    EndFile,
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
    _StartFile,
    _EndFile,
}

/// A Vim "Noun". This is the object following a verb (e.g. `dw`, `ciw`).
#[derive(Clone, Copy, Debug)]
pub enum Noun {
    Motion(Motion),
    InnerWord,    // "iw"
    InnerBigWord, // "iW"
    Word,
    BigWord,
    Sentence,
    Parentheses,
    Braces,
    Angles,
    Apostrophes,
    Quotes,
    Backtick,
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
            'i' => Some(SingleCommand::Insert),
            'I' => Some(SingleCommand::InsertSOL),
            'a' => Some(SingleCommand::Append),
            'A' => Some(SingleCommand::AppendEOL),
            'h' => Some(SingleCommand::MoveLeft),
            'l' => Some(SingleCommand::MoveRight),
            'j' => Some(SingleCommand::MoveDown),
            'k' => Some(SingleCommand::MoveUp),
            'w' => Some(SingleCommand::ForwardWord),
            'W' => Some(SingleCommand::ForwardBigWord),
            'b' => Some(SingleCommand::BackwardWord),
            'B' => Some(SingleCommand::BackwardBigWord),
            'x' => Some(SingleCommand::DeleteCharUnderCursor),
            'p' => Some(SingleCommand::Paste),
            '0' => Some(SingleCommand::StartFile),
            '$' => Some(SingleCommand::EndFile),
            _ => None,
        }
    }

    fn parse_multi_command(&mut self) -> Option<(MultiCommand, usize)> {
        match self.current() {
            // A single multi-command that doesn't need a Noun
            'C' => Some((MultiCommand::ChangeEOL, 2)),

            // `r` => next char is the replacement (like `rX`)
            'r' | 'R' => Some((MultiCommand::Replace(self.peek(1)?), 2)),

            // Operators that require a Noun
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
            'y' => Some((MultiCommand::Yank(Noun::Word), noun_length + 1)),
            _ => None,
        }
    }

    fn peek_noun(&self) -> Option<(Noun, usize)> {
        match self.peek(1)? {
            // Single-char motions:
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

    fn current(&self) -> char {
        self.peek(0).unwrap()
    }

    /// The `ahead`th zero-indexed character from the current position.
    /// `self.current()` is equivalent to `self.peek(0).unwrap()`.
    fn peek(&self, ahead: usize) -> Option<char> {
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
            Command::SingleCommand(single_command) => match single_command {
                SingleCommand::Insert
                | SingleCommand::Append
                | SingleCommand::AppendEOL
                | SingleCommand::InsertSOL
                | SingleCommand::DeleteCharUnderCursor
                | SingleCommand::Paste
                    if self.focus == Focus::Input =>
                {
                    match single_command {
                        SingleCommand::Insert => {
                            self.mode = Mode::Insert;
                        }
                        SingleCommand::Append => {
                            self.mode = Mode::Insert;
                            if self.cursor_pos < text.len() {
                                self.cursor_pos += 1;
                            }
                        }
                        SingleCommand::InsertSOL => {
                            self.mode = Mode::Insert;
                            self.cursor_pos = 0;
                        }
                        SingleCommand::AppendEOL => {
                            self.mode = Mode::Insert;
                            self.cursor_pos = text.len();
                        }
                        SingleCommand::DeleteCharUnderCursor => {
                            if self.cursor_pos < text.len() {
                                let removed_char = text.remove(self.cursor_pos);
                                let _ = clipboard
                                    .set_contents(removed_char.to_string());
                            }
                        }
                        SingleCommand::Paste => {
                            if let Ok(clip_text) = clipboard.get_contents() {
                                text.insert_str(self.cursor_pos, &clip_text);
                                self.cursor_pos += clip_text.len();
                            }
                        }
                        _ => {}
                    }
                }

                // Movement commands are allowed in *both* focuses, but do
                // different things
                SingleCommand::MoveLeft => {
                    if self.focus == Focus::Messages {
                        // Not implemented. Could do horizontal scroll in
                        // messages
                    } else if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                }
                SingleCommand::MoveRight => {
                    if self.focus == Focus::Messages {
                        // Not implemented
                    } else if self.cursor_pos < text.len() {
                        self.cursor_pos += 1;
                    }
                }
                SingleCommand::MoveUp => {
                    if self.focus == Focus::Messages {
                        self.scroll_offset =
                            self.scroll_offset.saturating_sub(1);
                    }
                }
                SingleCommand::MoveDown => {
                    if self.focus == Focus::Messages {
                        // Scroll down or move cursor
                        self.scroll_offset =
                            (self.scroll_offset + 1).min(height);
                    }
                }

                // Word motions (w/W/b/B)
                SingleCommand::ForwardWord
                | SingleCommand::ForwardBigWord
                | SingleCommand::BackwardWord
                | SingleCommand::BackwardBigWord => {
                    if self.focus == Focus::Input {
                        // Old logic for input
                        match single_command {
                            SingleCommand::ForwardWord => {
                                self.cursor_pos = find_next_word_boundary(
                                    text,
                                    self.cursor_pos,
                                );
                            }
                            SingleCommand::ForwardBigWord => {
                                self.cursor_pos = find_next_big_word_boundary(
                                    text,
                                    self.cursor_pos,
                                );
                            }
                            SingleCommand::BackwardWord => {
                                self.cursor_pos = find_prev_word_boundary(
                                    text,
                                    self.cursor_pos,
                                );
                            }
                            SingleCommand::BackwardBigWord => {
                                self.cursor_pos = find_prev_big_word_boundary(
                                    text,
                                    self.cursor_pos,
                                );
                            }
                            _ => {}
                        }
                    }
                }
                SingleCommand::StartFile => {
                    if self.focus == Focus::Messages {
                        self.scroll_offset = 0;
                    } else {
                        self.cursor_pos = 0;
                    }
                }
                SingleCommand::EndFile => {
                    if self.focus == Focus::Messages {
                        self.scroll_offset = height;
                    } else {
                        self.cursor_pos = text.len();
                    }
                }
                _ => {}
            },

            // -----------------------------
            // MultiCommand actions
            // -----------------------------
            Command::MultiCommand(multi_command) => {
                if self.focus == Focus::Input {
                    // Normal input editing
                    match multi_command {
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
        Noun::Motion(Motion::BackwardWord) => {
            let start_pos = find_prev_word_boundary(text, *cursor_pos);
            if start_pos < *cursor_pos {
                let removed =
                    text.drain(start_pos..*cursor_pos).collect::<String>();
                let _ = clipboard.set_contents(removed);
                *cursor_pos = start_pos;
            }
        }
        Noun::InnerWord => {
            // Example of deleting "inner word"
            todo!()
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
    if let Noun::Motion(Motion::ForwardWord) = noun {
        let end_pos = find_next_word_boundary(text, *cursor_pos);
        if end_pos > *cursor_pos {
            let selection = &text[*cursor_pos..end_pos];
            let _ = clipboard.set_contents(selection.to_string());
        }
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
        // find first match from remainder
        if let Some(mat) = regex.find(remainder) {
            let mut ms = mat.start();
            if let Some(ch) = remainder.chars().nth(ms) {
                if ch.is_whitespace() {
                    ms += 1;
                }
            }
            start_index + ms + (ms == 0) as usize
        } else {
            text.len()
        }
    } else {
        // find last match
        if let Some(mat) = matches.last() {
            let mut ms = mat.start();
            if let Some(ch) = remainder.chars().nth(ms) {
                if ch.is_whitespace() && ms > 0 {
                    ms -= 1;
                }
            }
            ms
        } else {
            0
        }
    }
}

fn find_next_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s\p{P}]", true)
}

fn find_next_big_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s]", true)
}

fn find_prev_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s\p{P}]", false)
}

fn find_prev_big_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s]", false)
}
