use copypasta::ClipboardProvider;

#[derive(Debug)]
pub enum Mode {
    Insert,
    Normal,
}

/// These are commands which work with a single keypress. These are the simplest
/// commands, like `i` for insert mode, `x` for delete char, etc.
pub enum SingleCmd {
    Insert,
    Append,
    InsertSOL, // start of line lol
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
}

/// These are commands which requires an additional vim clause. You need a word
/// object or motion to complete the command.
pub enum MultiCmd {
    Delete,
    Change,
    ChangeEOL,
    Replace(char),
    Yank,
}

/// A Command is a fully-specified action that can be applied to the editor state.
/// It may be a standalone command (like `i` for insert mode), or an operator
pub enum Command {
    SingleCmd(SingleCmd),
    MultiCmd(MultiCmd),
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
    x_pos: usize,
    operator: Option<Operator>,
}

impl<'a> VimCmd<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            x_pos: 0,
            operator: None,
        }
    }

    /// Consume the next character from `self.input`, if any.
    pub fn next_char(&mut self) -> Option<char> {
        let ch = self.input.chars().nth(self.x_pos);
        if ch.is_some() {
            self.x_pos += 1;
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
                    commands
                        .push(self.apply_operator_motion(op.clone(), motion));
                    // Operator is complete
                    self.operator = None;
                }
            } else {
                match ch {
                    // Standalone commands
                    'i' => commands.push(Command::SingleCmd(SingleCmd::Insert)),
                    'I' => {
                        commands.push(Command::SingleCmd(SingleCmd::InsertSOL))
                    }
                    'a' => commands.push(Command::SingleCmd(SingleCmd::Append)),
                    'A' => {
                        commands.push(Command::SingleCmd(SingleCmd::AppendEOL))
                    }
                    'h' => {
                        commands.push(Command::SingleCmd(SingleCmd::MoveLeft))
                    }
                    'l' => {
                        commands.push(Command::SingleCmd(SingleCmd::MoveRight))
                    }
                    'j' => {
                        commands.push(Command::SingleCmd(SingleCmd::MoveDown))
                    }
                    'k' => commands.push(Command::SingleCmd(SingleCmd::MoveUp)),
                    'w' => commands
                        .push(Command::SingleCmd(SingleCmd::ForwardWord)),
                    'W' => commands
                        .push(Command::SingleCmd(SingleCmd::ForwardBigWord)),
                    'b' => commands
                        .push(Command::SingleCmd(SingleCmd::BackwardWord)),
                    'B' => commands
                        .push(Command::SingleCmd(SingleCmd::BackwardBigWord)),
                    'x' => commands.push(Command::SingleCmd(
                        SingleCmd::DeleteCharUnderCursor,
                    )),
                    'p' => commands.push(Command::SingleCmd(SingleCmd::Paste)),

                    // Operator commands
                    'd' => self.operator = Some(Operator::Delete),
                    'c' => self.operator = Some(Operator::Change),
                    'y' => self.operator = Some(Operator::Yank),
                    'r' => {
                        if let Some(next_c) = self.next_char() {
                            commands.push(Command::MultiCmd(
                                MultiCmd::Replace(next_c),
                            ));
                        }
                    }
                    _ => {}
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
            Operator::Delete => Command::MultiCmd(MultiCmd::Delete),
            Operator::Change => match motion {
                Motion::ForwardWord
                | Motion::ForwardBigWord
                | Motion::BackwardWord
                | Motion::BackwardBigWord => {
                    Command::MultiCmd(MultiCmd::Change)
                }
                _ => Command::MultiCmd(MultiCmd::ChangeEOL),
            },
            Operator::Replace(c) => Command::MultiCmd(MultiCmd::Replace(c)),
            Operator::Yank => Command::MultiCmd(MultiCmd::Yank),
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
        message_pos: &mut u16,
        height: u16,
        text: &mut String,
        clipboard: &mut copypasta::ClipboardContext,
        commands: Vec<Command>,
    ) {
        for cmd in commands {
            match cmd {
                Command::SingleCmd(single_cmd) => match single_cmd {
                    SingleCmd::Insert => {
                        *mode = Mode::Insert;
                    }
                    SingleCmd::Append => {
                        *mode = Mode::Insert;
                        if *cursor_pos < text.len() {
                            *cursor_pos += 1;
                        }
                    }
                    SingleCmd::InsertSOL => {
                        *mode = Mode::Insert;
                        *cursor_pos = 0;
                    }
                    SingleCmd::AppendEOL => {
                        *mode = Mode::Insert;
                        *cursor_pos = text.len();
                    }
                    SingleCmd::DeleteCharUnderCursor => {
                        if *cursor_pos < text.len() {
                            let removed_char = text.remove(*cursor_pos);
                            let _ = clipboard
                                .set_contents(removed_char.to_string());
                        }
                    }
                    SingleCmd::MoveLeft => {
                        if *cursor_pos > 0 {
                            *cursor_pos -= 1;
                        }
                    }
                    SingleCmd::MoveRight => {
                        if *cursor_pos < text.len() {
                            *cursor_pos += 1;
                        }
                    }
                    SingleCmd::MoveUp => {
                        *message_pos = message_pos.saturating_sub(1);
                    }
                    SingleCmd::MoveDown => {
                        *message_pos = (*message_pos + 1).min(height);
                    }
                    SingleCmd::ForwardWord => {
                        *cursor_pos =
                            find_next_word_boundary(text, *cursor_pos);
                    }
                    SingleCmd::ForwardBigWord => {
                        *cursor_pos =
                            find_next_big_word_boundary(text, *cursor_pos);
                    }
                    SingleCmd::BackwardWord => {
                        *cursor_pos =
                            find_prev_word_boundary(text, *cursor_pos);
                    }
                    SingleCmd::BackwardBigWord => {
                        *cursor_pos =
                            find_prev_big_word_boundary(text, *cursor_pos);
                    }
                    SingleCmd::Paste => {
                        if let Ok(clip_text) = clipboard.get_contents() {
                            text.insert_str(*cursor_pos, &clip_text);
                            *cursor_pos += clip_text.len();
                        }
                    }
                },
                Command::MultiCmd(multi_cmd) => match multi_cmd {
                    MultiCmd::Delete => {
                        let word_end =
                            find_next_word_boundary(text, *cursor_pos);
                        if word_end > *cursor_pos {
                            let removed = text
                                .drain(*cursor_pos..word_end)
                                .collect::<String>();
                            let _ = clipboard.set_contents(removed);
                        }
                    }
                    MultiCmd::Change => {
                        let word_end =
                            find_next_word_boundary(text, *cursor_pos);
                        if word_end > *cursor_pos {
                            let removed = text
                                .drain(*cursor_pos..word_end)
                                .collect::<String>();
                            let _ = clipboard.set_contents(removed);
                        }
                        *mode = Mode::Insert;
                    }
                    MultiCmd::ChangeEOL => {
                        let end_of_line = text.len();
                        if *cursor_pos < end_of_line {
                            let removed = text
                                .drain(*cursor_pos..end_of_line)
                                .collect::<String>();
                            let _ = clipboard.set_contents(removed);
                        }
                        *mode = Mode::Insert;
                    }
                    MultiCmd::Replace(c) => {
                        if *cursor_pos < text.len() {
                            let removed_char = text.remove(*cursor_pos);
                            let _ = clipboard
                                .set_contents(removed_char.to_string());
                            text.insert(*cursor_pos, c);
                        }
                    }
                    MultiCmd::Yank => {
                        let word_end =
                            find_next_word_boundary(text, *cursor_pos);
                        if word_end > *cursor_pos {
                            let substring = &text[*cursor_pos..word_end];
                            let _ =
                                clipboard.set_contents(substring.to_string());
                        }
                    }
                },
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
