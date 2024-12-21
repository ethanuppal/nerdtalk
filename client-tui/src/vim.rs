use copypasta::ClipboardProvider;
use regex::Regex;

#[derive(Debug)]
pub enum Mode {
    Insert,
    Normal,
}

/// These are commands which work with a single keypress. These are the simplest
/// commands, like `i` for insert mode, `x` for delete char, etc.
pub enum SingleCommand {
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
pub enum MultiCommand {
    Delete,
    Change,
    ChangeEOL,
    Replace(char),
    Yank,
}

/// A Command is a fully-specified action that can be applied to the editor state.
/// It may be a standalone command (like `i` for insert mode), or an operator
pub enum Command {
    SingleCommand(SingleCommand),
    MultiCommand(MultiCommand),
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
pub struct VimCommand<'a> {
    input: &'a str,
    x_pos: usize,
    operator: Option<Operator>,
}

impl Iterator for VimCommand<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let ch = self.input.chars().nth(self.x_pos);
        if ch.is_some() {
            self.x_pos += 1;
        }
        ch
    }
}

impl<'a> VimCommand<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            x_pos: 0,
            operator: None,
        }
    }

    /// Parse the entire input buffer into zero or more Commands.
    pub fn parse(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();

        while let Some(ch) = self.next() {
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
                    'i' => commands.push(Command::SingleCommand(SingleCommand::Insert)),
                    'I' => {
                        commands.push(Command::SingleCommand(SingleCommand::InsertSOL))
                    }
                    'a' => commands.push(Command::SingleCommand(SingleCommand::Append)),
                    'A' => {
                        commands.push(Command::SingleCommand(SingleCommand::AppendEOL))
                    }
                    'h' => {
                        commands.push(Command::SingleCommand(SingleCommand::MoveLeft))
                    }
                    'l' => {
                        commands.push(Command::SingleCommand(SingleCommand::MoveRight))
                    }
                    'j' => {
                        commands.push(Command::SingleCommand(SingleCommand::MoveDown))
                    }
                    'k' => commands.push(Command::SingleCommand(SingleCommand::MoveUp)),
                    'w' => commands
                        .push(Command::SingleCommand(SingleCommand::ForwardWord)),
                    'W' => commands
                        .push(Command::SingleCommand(SingleCommand::ForwardBigWord)),
                    'b' => commands
                        .push(Command::SingleCommand(SingleCommand::BackwardWord)),
                    'B' => commands
                        .push(Command::SingleCommand(SingleCommand::BackwardBigWord)),
                    'x' => commands.push(Command::SingleCommand(
                        SingleCommand::DeleteCharUnderCursor,
                    )),
                    'p' => commands.push(Command::SingleCommand(SingleCommand::Paste)),

                    // Operator commands
                    'd' => self.operator = Some(Operator::Delete),
                    'c' => self.operator = Some(Operator::Change),
                    'y' => self.operator = Some(Operator::Yank),
                    'r' => {
                        if let Some(next_c) = self.next() {
                            commands.push(Command::MultiCommand(
                                MultiCommand::Replace(next_c),
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
            Operator::Delete => Command::MultiCommand(MultiCommand::Delete),
            Operator::Change => match motion {
                Motion::ForwardWord
                | Motion::ForwardBigWord
                | Motion::BackwardWord
                | Motion::BackwardBigWord => {
                    Command::MultiCommand(MultiCommand::Change)
                }
                _ => Command::MultiCommand(MultiCommand::ChangeEOL),
            },
            Operator::Replace(c) => Command::MultiCommand(MultiCommand::Replace(c)),
            Operator::Yank => Command::MultiCommand(MultiCommand::Yank),
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
        undo_stack: &mut Vec<String>,
        commands: Vec<Command>,
    ) {
        for cmd in commands {
            match cmd {
                // Good
                Command::SingleCommand(single_cmd) => match single_cmd {
                    SingleCommand::Insert => {
                        *mode = Mode::Insert;
                    }
                    SingleCommand::Append => {
                        *mode = Mode::Insert;
                        if *cursor_pos < text.len() {
                            *cursor_pos += 1;
                        }
                    }
                    SingleCommand::InsertSOL => {
                        *mode = Mode::Insert;
                        *cursor_pos = 0;
                    }
                    SingleCommand::AppendEOL => {
                        *mode = Mode::Insert;
                        *cursor_pos = text.len();
                    }
                    SingleCommand::DeleteCharUnderCursor => {
                        if *cursor_pos < text.len() {
                            let removed_char = text.remove(*cursor_pos);
                            let _ = clipboard
                                .set_contents(removed_char.to_string());
                        }
                    }
                    SingleCommand::MoveLeft => {
                        if *cursor_pos > 0 {
                            *cursor_pos -= 1;
                        }
                    }
                    SingleCommand::MoveRight => {
                        if *cursor_pos < text.len() {
                            *cursor_pos += 1;
                        }
                    }
                    SingleCommand::MoveUp => {
                        *message_pos = message_pos.saturating_sub(1);
                    }
                    SingleCommand::MoveDown => {
                        *message_pos = (*message_pos + 1).min(height);
                    }
                    SingleCommand::ForwardWord => {
                        *cursor_pos =
                            find_next_word_boundary(text, *cursor_pos);
                    }
                    SingleCommand::ForwardBigWord => {
                        *cursor_pos =
                            find_next_big_word_boundary(text, *cursor_pos);
                    }
                    SingleCommand::BackwardWord => {
                        *cursor_pos =
                            find_prev_word_boundary(text, *cursor_pos);
                    }
                    SingleCommand::BackwardBigWord => {
                        *cursor_pos =
                            find_prev_big_word_boundary(text, *cursor_pos);
                    }
                    SingleCommand::Paste => {
                        if let Ok(clip_text) = clipboard.get_contents() {
                            text.insert_str(*cursor_pos, &clip_text);
                            *cursor_pos += clip_text.len();
                        }
                    }
                },
                Command::MultiCommand(multi_cmd) => match multi_cmd {
                    MultiCommand::Delete => {
                        let word_end =
                            find_next_word_boundary(text, *cursor_pos);
                        if word_end > *cursor_pos {
                            let removed = text
                                .drain(*cursor_pos..word_end)
                                .collect::<String>();
                            let _ = clipboard.set_contents(removed);
                        }
                    }
                    MultiCommand::Change => {
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
                    MultiCommand::ChangeEOL => {
                        let end_of_line = text.len();
                        if *cursor_pos < end_of_line {
                            let removed = text
                                .drain(*cursor_pos..end_of_line)
                                .collect::<String>();
                            let _ = clipboard.set_contents(removed);
                        }
                        *mode = Mode::Insert;
                    }
                    MultiCommand::Replace(c) => {
                        if *cursor_pos < text.len() {
                            let removed_char = text.remove(*cursor_pos);
                            let _ = clipboard
                                .set_contents(removed_char.to_string());
                            text.insert(*cursor_pos, c);
                        }
                    }
                    MultiCommand::Yank => {
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

/// TODO: Spaces following punctuation aren't avoided. Need to fix this.
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
        if let Some(mat) = regex.find(remainder) {
            let mut ms = mat.start();
            if char::is_whitespace(remainder.chars().nth(ms).unwrap()) {
                ms = ms + 1;
            }
            start_index + ms + (ms == 0) as usize
        } else {
            text.len()
        }
    } else {
        if let Some(mat) = matches.last() {
            let mut ms = mat.start();
            if char::is_whitespace(remainder.chars().nth(ms).unwrap()) {
                ms = if ms > 0 { ms - 1 } else { 0 };
            }
            ms
        } else {
            0
        }
    }
}

/// Find the next "word boundary" from `start`.
fn find_next_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s\p{P}]", true)
}

/// A bigger word boundary might consider punctuation, multiple spaces, etc.
fn find_next_big_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s]", true)
}

/// Move backwards to the previous word boundary (space).
fn find_prev_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s\p{P}]", false)
}

/// A "big word" backward boundary might skip punctuation, etc.
fn find_prev_big_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s]", false)
}
