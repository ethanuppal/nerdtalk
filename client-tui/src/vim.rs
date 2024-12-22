use copypasta::ClipboardProvider;
use regex::Regex;

/// Different Vim Modes
#[derive(Debug)]
pub enum Mode {
    Insert,
    Normal,
}

/// These are commands which work with a single keypress.
/// For example: `i` (insert), `x` (delete char), `h` (move left).
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
#[derive(Clone, Debug)]
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

/// A Vim "Noun". This is the object following a verb, e.g. `dw` or `ciw`.
/// Below are a few examples. In practice, you'd have many more variants
/// (e.g. "aW" => "around Big Word", etc.).
#[derive(Clone, Debug)]
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

/// Our "operators" can embed a `Noun`.
/// For example, `Delete(Noun::Motion(Motion::ForwardWord))` => `dw`.
#[derive(Clone, Debug)]
pub enum MultiCommand {
    Delete(Noun),
    Change(Noun),
    ChangeEOL,
    Replace(char),
    Yank(Noun),
}

/// A high-level `Command` is either:
///  - A single-key command (`SingleCommand`),
///  - Or a fully-specified operator + object (`MultiCommand`).
#[derive(Clone, Debug)]
pub enum Command {
    SingleCommand(SingleCommand),
    MultiCommand(MultiCommand),
}

/// Simple [`VimCommand`] parser. Tracks an optional (pending) operator.
pub struct VimCommand<'a> {
    input: &'a str,
    x_pos: usize,
    operator: Option<MultiCommand>,
}

impl<'a> VimCommand<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            x_pos: 0,
            operator: None,
        }
    }
}

/// Implement `Iterator` so we can easily consume the input one character at a time.
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
    /// Parse the entire input buffer into zero or more [`Command`]s.
    pub fn parse(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();

        while let Some(ch) = self.next() {
            // If we already have a pending operator (like `Delete(...)`),
            // we try to parse the next character(s) as a Noun.
            if let Some(op) = &self.operator.clone() {
                if let Some(noun) = self.parse_noun(ch) {
                    // We successfully parsed a Noun => finalize the MultiCommand.
                    let cmd = match op {
                        // Convert the "pending" operator + noun => final command
                        MultiCommand::Delete(_) => {
                            Command::MultiCommand(MultiCommand::Delete(noun))
                        }
                        MultiCommand::Change(_) => {
                            Command::MultiCommand(MultiCommand::Change(noun))
                        }
                        MultiCommand::Yank(_) => {
                            Command::MultiCommand(MultiCommand::Yank(noun))
                        }
                        // If the operator was `ChangeEOL` or `Replace(_)`,
                        // that's a special case we might handle differently.
                        // We'll just do nothing special here.
                        MultiCommand::ChangeEOL => {
                            Command::MultiCommand(MultiCommand::ChangeEOL)
                        }
                        MultiCommand::Replace(_) => {
                            // In theory, `Replace` was waiting for a single char
                            // (like `rX`). If we get a motion here, it's not expected.
                            // We'll skip or handle an error.
                            continue;
                        }
                    };
                    commands.push(cmd);
                    self.operator = None;
                } else {
                    match op {
                        MultiCommand::Replace(_) => {
                            let replace_cmd = Command::MultiCommand(
                                MultiCommand::Replace(ch),
                            );
                            commands.push(replace_cmd);
                            self.operator = None;
                        }
                        _ => {
                            // If it's a different operator (like d, y, c)
                            // but we got an unknown char => skip or ignore
                            self.operator = None;
                        }
                    }
                }
            } else {
                // No operator is pending => either a single-key command or the start of an operator
                match ch {
                    // Standalone commands
                    'i' => commands
                        .push(Command::SingleCommand(SingleCommand::Insert)),
                    'I' => commands
                        .push(Command::SingleCommand(SingleCommand::InsertSOL)),
                    'a' => commands
                        .push(Command::SingleCommand(SingleCommand::Append)),
                    'A' => commands
                        .push(Command::SingleCommand(SingleCommand::AppendEOL)),
                    'h' => commands
                        .push(Command::SingleCommand(SingleCommand::MoveLeft)),
                    'l' => commands
                        .push(Command::SingleCommand(SingleCommand::MoveRight)),
                    'j' => commands
                        .push(Command::SingleCommand(SingleCommand::MoveDown)),
                    'k' => commands
                        .push(Command::SingleCommand(SingleCommand::MoveUp)),
                    'w' => commands.push(Command::SingleCommand(
                        SingleCommand::ForwardWord,
                    )),
                    'W' => commands.push(Command::SingleCommand(
                        SingleCommand::ForwardBigWord,
                    )),
                    'b' => commands.push(Command::SingleCommand(
                        SingleCommand::BackwardWord,
                    )),
                    'B' => commands.push(Command::SingleCommand(
                        SingleCommand::BackwardBigWord,
                    )),
                    'x' => commands.push(Command::SingleCommand(
                        SingleCommand::DeleteCharUnderCursor,
                    )),
                    'p' => commands
                        .push(Command::SingleCommand(SingleCommand::Paste)),
                    '0' => commands
                        .push(Command::SingleCommand(SingleCommand::StartFile)),
                    '$' => commands
                        .push(Command::SingleCommand(SingleCommand::EndFile)),

                    // Operators that require a Noun
                    'd' => {
                        self.operator = Some(MultiCommand::Delete(Noun::Word));
                    }
                    'c' => {
                        self.operator = Some(MultiCommand::Change(Noun::Word));
                    }
                    'y' => {
                        self.operator = Some(MultiCommand::Yank(Noun::Word));
                    }

                    // A single multi-command that doesn't need a Noun
                    'C' => {
                        commands.push(Command::MultiCommand(
                            MultiCommand::ChangeEOL,
                        ));
                    }

                    // `r` => next char is the replacement (like `rX`)
                    'r' | 'R' => {
                        self.operator = Some(MultiCommand::Replace('\0'));
                    }

                    _ => {}
                }
            }
        }

        commands
    }

    pub fn is_operator_pending(&self) -> bool {
        self.operator.is_some()
    }

    /// Attempt to parse a 1- or 2-char Noun, given the first char `ch`.
    /// Returns `Some(Noun)` if we recognized it, or `None` otherwise.
    fn parse_noun(&mut self, ch: char) -> Option<Noun> {
        match ch {
            // Single-char motions:
            'w' => Some(Noun::Motion(Motion::ForwardWord)),
            'W' => Some(Noun::Motion(Motion::ForwardBigWord)),
            'b' => Some(Noun::Motion(Motion::BackwardWord)),
            'B' => Some(Noun::Motion(Motion::BackwardBigWord)),
            'h' => Some(Noun::Motion(Motion::Left)),
            'l' => Some(Noun::Motion(Motion::Right)),
            'j' => Some(Noun::Motion(Motion::Down)),
            'k' => Some(Noun::Motion(Motion::Up)),

            // TODO: Change this a bit when I add 'aw' type of cmds
            'i' => {
                if let Some(next_ch) = self.next() {
                    return match next_ch {
                        'w' => Some(Noun::InnerWord),
                        'W' => Some(Noun::InnerBigWord),
                        's' => Some(Noun::Sentence),
                        '(' | ')' => Some(Noun::Parentheses),
                        '[' | ']' => Some(Noun::Braces),
                        '{' | '}' => Some(Noun::Braces),
                        '<' | '>' => Some(Noun::Angles),
                        '\'' => Some(Noun::Apostrophes),
                        '"' => Some(Noun::Quotes),
                        '`' => Some(Noun::Backtick),
                        _ => None,
                    };
                }
                None
            }
            _ => None,
        }
    }

    /// Applies a list of [`Command`]s to the current editor state.
    /// (Where you implement the actual editing logic.)
    pub fn apply_cmds(
        &mut self,
        mode: &mut Mode,
        cursor_pos: &mut usize,
        message_pos: &mut u16,
        height: u16,
        text: &mut String,
        clipboard: &mut copypasta::ClipboardContext,
        _undo_stack: &mut Vec<String>,
        commands: Vec<Command>,
    ) {
        for cmd in commands {
            match cmd {
                // -----------------------------
                // SingleCommand actions
                // -----------------------------
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
                    SingleCommand::StartFile => *cursor_pos = 0,
                    SingleCommand::EndFile => *cursor_pos = text.len(),
                },

                // -----------------------------
                // MultiCommand actions
                // -----------------------------
                Command::MultiCommand(multi_cmd) => match multi_cmd {
                    MultiCommand::Delete(noun) => {
                        delete_helper(cursor_pos, text, clipboard, &noun);
                    }
                    MultiCommand::Change(noun) => {
                        handle_change(mode, cursor_pos, text, clipboard, &noun);
                    }
                    MultiCommand::Yank(noun) => {
                        yank_helper(cursor_pos, text, clipboard, &noun);
                    }
                    MultiCommand::ChangeEOL => {
                        if *cursor_pos < text.len() {
                            let removed =
                                text.drain(*cursor_pos..).collect::<String>();
                            let _ = clipboard.set_contents(removed);
                        }
                        *mode = Mode::Insert;
                    }
                    MultiCommand::Replace(c) => {
                        if *cursor_pos < text.len() {
                            text.remove(*cursor_pos);
                            text.insert(*cursor_pos, c);
                        }
                    }
                },
            }
        }
    }
}

fn delete_helper(
    cursor_pos: &mut usize,
    text: &mut String,
    clipboard: &mut copypasta::ClipboardContext,
    noun: &Noun,
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
            todo!()
        }
        _ => {}
    }
}

fn handle_change(
    mode: &mut Mode,
    cursor_pos: &mut usize,
    text: &mut String,
    clipboard: &mut copypasta::ClipboardContext,
    noun: &Noun,
) {
    delete_helper(cursor_pos, text, clipboard, noun);
    *mode = Mode::Insert;
}

/// Example: handle “yank” + motion
fn yank_helper(
    cursor_pos: &mut usize,
    text: &mut String,
    clipboard: &mut copypasta::ClipboardContext,
    noun: &Noun,
) {
    match noun {
        // e.g. "yw" => yank until next word boundary
        Noun::Motion(Motion::ForwardWord) => {
            let end_pos = find_next_word_boundary(text, *cursor_pos);
            if end_pos > *cursor_pos {
                let selection = &text[*cursor_pos..end_pos];
                let _ = clipboard.set_contents(selection.to_string());
            }
        }
        // ...
        _ => {}
    }
}

// --------------------- Word boundary helpers -----------------------

/// A helper that finds a boundary (regex-based).
/// This is simplistic and won't handle all edge cases.
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
                ms += 1;
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

/// Move backwards to the previous word boundary.
fn find_prev_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s\p{P}]", false)
}

/// A "big word" backward boundary might skip punctuation, etc.
fn find_prev_big_word_boundary(text: &str, start: usize) -> usize {
    word_boundary(text, start, r"[\s]", false)
}
