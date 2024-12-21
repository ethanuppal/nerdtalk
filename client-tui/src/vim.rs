use copypasta::ClipboardProvider;

#[derive(Debug)]
pub enum Mode {
    Insert,
    Normal,
}

pub enum Command {
    // Standalone commands
    Insert,
    Append,
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

    // Operator-pending commands
    Delete,
    Change,
    ChangeEOL,
    Replace(char),
    Yank,
}

/// The Operator describes a partially-specified command that requires a Motion.
#[derive(Clone)]
pub enum Operator {
    Delete,
    Change,
    Replace(char),
    Yank,
}

/// A Motion indicates movement over some text, e.g. `w`, `b`, `h`, etc.
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    ForwardWord,
    ForwardBigWord,
    BackwardWord,
    BackwardBigWord,
}

/// Simple VimCmd parser. Tracks an optional operator.
pub struct VimCmd<'a> {
    input: &'a str,
    pos: usize,
    operator: Option<Operator>,
}

impl<'a> VimCmd<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            operator: None,
        }
    }

    /// Consume the next character from `self.input`, if any.
    pub fn next_char(&mut self) -> Option<char> {
        let ch = self.input.chars().nth(self.pos);
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    /// Parse the entire input buffer into zero or more Commands.
    pub fn parse(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();

        while let Some(ch) = self.next_char() {
            if let Some(op) = &self.operator {
                // We already have an operator: interpret next char as a motion or text object
                if let Some(motion) = self.char_to_motion(ch) {
                    // We have op + motion => full command
                    commands.push(self.apply_operator_motion(op.clone(), motion));
                    // Operator is complete
                    self.operator = None;
                } else {
                    // Possibly more logic for 'f<char>', 'i<char>' text objects, etc.
                    // For simplicity, do nothing unless it's a recognized motion.
                }
            } else {
                match ch {
                    // Standalone commands
                    'i' => commands.push(Command::Insert),
                    'a' => commands.push(Command::Append),
                    'A' => commands.push(Command::AppendEOL),
                    'h' => commands.push(Command::MoveLeft),
                    'l' => commands.push(Command::MoveRight),
                    'j' => commands.push(Command::MoveDown),
                    'k' => commands.push(Command::MoveUp),
                    'w' => commands.push(Command::ForwardWord),
                    'W' => commands.push(Command::ForwardBigWord),
                    'b' => commands.push(Command::BackwardWord),
                    'B' => commands.push(Command::BackwardBigWord),
                    'x' => {
                        // By convention in Vim, `x` is like `dl` (delete char under cursor).
                        // We'll treat it as a one-char delete. We'll also put that text in clipboard.
                        commands.push(Command::DeleteCharUnderCursor);
                    }
                    'p' => {
                        // Paste from the clipboard
                        commands.push(Command::Paste);
                    }

                    // Operator commands
                    'd' => self.operator = Some(Operator::Delete),
                    'c' => self.operator = Some(Operator::Change),
                    'y' => self.operator = Some(Operator::Yank),
                    'r' => {
                        // 'r' requires a character to replace with
                        if let Some(next_c) = self.next_char() {
                            self.operator = Some(Operator::Replace(next_c));
                            // 'rX' => immediately replace char under cursor with X
                            commands.push(Command::Replace(next_c));
                            self.operator = None;
                        }
                    }

                    _ => {
                        // Unhandled command or just ignore
                    }
                }
            }
        }

        commands
    }

    /// Return true if we have an operator still pending, false otherwise.
    pub fn is_operator_pending(&self) -> bool {
        self.operator.is_some()
    }

    /// Given an Operator + Motion, produce the final Command.
    fn apply_operator_motion(&self, op: Operator, motion: Motion) -> Command {
        match op {
            Operator::Delete => {
                // e.g. `d w` => "Delete forward word"
                // You could create a new Command::DeleteMotion(motion), or expand as you see fit.
                match motion {
                    Motion::ForwardWord
                    | Motion::ForwardBigWord
                    | Motion::BackwardWord
                    | Motion::BackwardBigWord => Command::Delete,
                    _ => Command::Delete,
                }
            }
            Operator::Change => {
                // e.g. `c w` => "Change forward word"
                match motion {
                    Motion::ForwardWord
                    | Motion::ForwardBigWord
                    | Motion::BackwardWord
                    | Motion::BackwardBigWord => Command::Change,
                    _ => Command::ChangeEOL, // as a fallback
                }
            }
            Operator::Replace(c) => Command::Replace(c),
            Operator::Yank => {
                // e.g. `y w` => "Yank forward word"
                // We'll just treat it all as yank for simplicity
                Command::Yank
            }
        }
    }

    /// Map char to Motion if recognized, or None if not.
    fn char_to_motion(&self, ch: char) -> Option<Motion> {
        match ch {
            'w' => Some(Motion::ForwardWord),
            'W' => Some(Motion::ForwardBigWord),
            'b' => Some(Motion::BackwardWord),
            'B' => Some(Motion::BackwardBigWord),
            'h' => Some(Motion::Left),
            'l' => Some(Motion::Right),
            'j' => Some(Motion::Down),
            'k' => Some(Motion::Up),
            _ => None,
        }
    }

    /// Applies a list of Commands to the current editor state.  
    /// This is where you do the actual editing (insert/delete/yank/paste) logic.
    pub fn apply_cmds(
        &self,
        mode: &mut Mode,
        cursor_pos: &mut usize,
        text: &mut String,
        clipboard: &mut copypasta::ClipboardContext,
        commands: Vec<Command>,
    ) {
        for cmd in commands {
            match cmd {
                // -------------------------------
                // 1) Standalone commands
                // -------------------------------
                Command::Insert => {
                    *mode = Mode::Insert;
                }
                Command::Append => {
                    *mode = Mode::Insert;
                    // Move cursor one char right if possible
                    if *cursor_pos < text.len() {
                        *cursor_pos += 1;
                    }
                }
                Command::AppendEOL => {
                    *mode = Mode::Insert;
                    *cursor_pos = text.len();
                }
                Command::DeleteCharUnderCursor => {
                    if *cursor_pos < text.len() {
                        // Grab the char being deleted for the clipboard
                        let removed_char = text.remove(*cursor_pos);
                        let _ = clipboard.set_contents(removed_char.to_string());
                    }
                }
                Command::MoveLeft => {
                    if *cursor_pos > 0 {
                        *cursor_pos -= 1;
                    }
                }
                Command::MoveRight => {
                    if *cursor_pos < text.len() {
                        *cursor_pos += 1;
                    }
                }
                Command::MoveUp => {
                    // For multi-line text, you'd do more complex logic here.
                    // As a placeholder, let's move the cursor up ~10 chars if possible.
                    let up_offset = 10_usize.min(*cursor_pos);
                    *cursor_pos = cursor_pos.saturating_sub(up_offset);
                }
                Command::MoveDown => {
                    // Similarly, move the cursor down ~10 chars as a placeholder.
                    let down_offset = 10_usize;
                    *cursor_pos = (*cursor_pos + down_offset).min(text.len());
                }
                Command::ForwardWord => {
                    *cursor_pos = find_next_word_boundary(text, *cursor_pos);
                }
                Command::ForwardBigWord => {
                    *cursor_pos = find_next_big_word_boundary(text, *cursor_pos);
                }
                Command::BackwardWord => {
                    *cursor_pos = find_prev_word_boundary(text, *cursor_pos);
                }
                Command::BackwardBigWord => {
                    *cursor_pos = find_prev_big_word_boundary(text, *cursor_pos);
                }
                Command::Paste => {
                    if let Ok(clip_text) = clipboard.get_contents() {
                        // Insert the clipboard contents at the cursor
                        text.insert_str(*cursor_pos, &clip_text);
                        *cursor_pos += clip_text.len();
                    }
                }

                // -------------------------------
                // 2) Operator-pending commands
                // -------------------------------
                Command::Delete => {
                    // Example: "d w" => delete a word.
                    // For simplicity, let's assume 'delete' means a small range, or entire line, etc.
                    // We'll demonstrate a simple "delete next word" approach:
                    let word_end = find_next_word_boundary(text, *cursor_pos);
                    if word_end > *cursor_pos {
                        let removed = text.drain(*cursor_pos..word_end).collect::<String>();
                        let _ = clipboard.set_contents(removed);
                    }
                }
                Command::Change => {
                    // Example: "c w" => change a word (delete + go insert)
                    let word_end = find_next_word_boundary(text, *cursor_pos);
                    if word_end > *cursor_pos {
                        let removed = text.drain(*cursor_pos..word_end).collect::<String>();
                        let _ = clipboard.set_contents(removed);
                    }
                    *mode = Mode::Insert;
                }
                Command::ChangeEOL => {
                    let end_of_line = text.len();
                    if *cursor_pos < end_of_line {
                        let removed = text.drain(*cursor_pos..end_of_line).collect::<String>();
                        let _ = clipboard.set_contents(removed);
                    }
                    *mode = Mode::Insert;
                }
                Command::Replace(c) => {
                    // Replace the character under the cursor
                    if *cursor_pos < text.len() {
                        // The removed character goes to clipboard too (like 'x')
                        let removed_char = text.remove(*cursor_pos);
                        let _ = clipboard.set_contents(removed_char.to_string());

                        // Insert the new char
                        text.insert(*cursor_pos, c);
                    }
                }
                Command::Yank => {
                    let word_end = find_next_word_boundary(text, *cursor_pos);
                    if word_end > *cursor_pos {
                        let substring = &text[*cursor_pos..word_end];
                        let _ = clipboard.set_contents(substring.to_string());
                    }
                }
            }
        }
    }
}

/// Find the next "word boundary" from `start`. 
/// This is very naive: it scans until it sees a space or end of string.
fn find_next_word_boundary(text: &str, start: usize) -> usize {
    let remainder = &text[start..];
    if let Some(next_space) = remainder.find(char::is_whitespace) {
        start + next_space
    } else {
        text.len()
    }
}

/// A bigger word boundary might consider punctuation, multiple spaces, etc.
/// For demonstration, let's skip until next whitespace or end of text.
fn find_next_big_word_boundary(text: &str, start: usize) -> usize {
    // We treat big word boundary the same as normal forward word for now,
    // but you can expand this to skip punctuation, etc.
    find_next_word_boundary(text, start)
}

/// Move backwards to the previous word boundary (space).
fn find_prev_word_boundary(text: &str, start: usize) -> usize {
    if start == 0 {
        return 0;
    }
    let slice = &text[..start];
    if let Some(idx) = slice.rfind(char::is_whitespace) {
        idx
    } else {
        0
    }
}

/// A "big word" backward boundary might skip punctuation, etc. 
fn find_prev_big_word_boundary(text: &str, start: usize) -> usize {
    // Same as normal backward word for demonstration
    find_prev_word_boundary(text, start)
}