//! Native command-line helpers for the SIS command package.
//!
//! The legacy file combined terminal-specific file completion with prompt and
//! history substitution helpers. This port keeps the terminal boundary outside
//! the module and exposes deterministic operations that callers can wire to an
//! interactive shell.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_HISTORY_CHAR: char = '%';
pub const DEFAULT_SUBSTITUTE_CHAR: char = '^';
pub const DEFAULT_SEPARATORS: &str = " \t\n;";
pub const BEEP: char = '\u{7}';
pub const ESCAPE: char = '\u{1b}';

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistorySubstitutionOptions {
    pub history_char: char,
    pub substitute_char: char,
    pub separators: String,
}

impl Default for HistorySubstitutionOptions {
    fn default() -> Self {
        Self {
            history_char: DEFAULT_HISTORY_CHAR,
            substitute_char: DEFAULT_SUBSTITUTE_CHAR,
            separators: DEFAULT_SEPARATORS.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistorySubstitutionResult {
    pub line: String,
    pub changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistorySubstitutionError {
    ModifierFailed,
    EventNotFound(i32),
    PrefixNotFound(String),
    BadArgumentSelector(char),
}

impl fmt::Display for HistorySubstitutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModifierFailed => f.write_str("Modifier failed"),
            Self::EventNotFound(index) => write!(f, "Event {index} not found"),
            Self::PrefixNotFound(prefix) => write!(f, "Event not found: {prefix}"),
            Self::BadArgumentSelector(history_char) => {
                write!(f, "Bad {history_char} arg selector")
            }
        }
    }
}

impl Error for HistorySubstitutionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileCompletionRequest {
    Complete,
    List,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileCompletionResult {
    pub line: String,
    pub listing: Vec<String>,
    pub beep: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionWord {
    pub prefix_start: usize,
    pub directory_text: String,
    pub file_prefix: String,
}

pub fn render_prompt(prompt: Option<&str>, history_len: usize, history_char: char) -> String {
    let Some(prompt) = prompt else {
        return String::new();
    };

    let next_history_index = (history_len + 1).to_string();
    let mut rendered = String::new();

    for ch in prompt.chars() {
        if ch == history_char {
            rendered.push_str(&next_history_index);
        } else {
            rendered.push(ch);
        }
    }

    rendered
}

pub fn substitute_history(
    line: &str,
    history: &[String],
    options: &HistorySubstitutionOptions,
) -> Result<HistorySubstitutionResult, HistorySubstitutionError> {
    let line = line.trim_start_matches(char::is_whitespace);
    if line.is_empty() {
        return Ok(HistorySubstitutionResult {
            line: line.to_owned(),
            changed: false,
        });
    }

    let last = history.last().map(String::as_str).unwrap_or("");
    if line.starts_with(options.substitute_char) {
        return substitute_previous_command(line, last, options.substitute_char);
    }

    let mut output = String::new();
    let mut changed = false;
    let mut internal_change = false;
    let mut chars = line.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if ch != options.history_char {
            output.push(ch);
            continue;
        }

        if line[..index].ends_with('\\') {
            output.pop();
            output.push(options.history_char);
            internal_change = true;
            continue;
        }

        if history.is_empty() {
            return Err(HistorySubstitutionError::EventNotFound(0));
        }

        let Some((_, selector)) = chars.next() else {
            return Err(HistorySubstitutionError::PrefixNotFound(String::new()));
        };

        match selector {
            value if value == options.history_char => {
                output.push_str(last);
            }
            '$' => {
                output.push_str(&argument_selector(last, -1, &options.separators)?);
            }
            '*' => {
                output.push_str(&argument_selector(last, -2, &options.separators)?);
            }
            ':' => {
                let number = read_number(&mut chars);
                let argument =
                    argument_selector(last, number, &options.separators).map_err(|_| {
                        HistorySubstitutionError::BadArgumentSelector(options.history_char)
                    })?;
                output.push_str(&argument);
            }
            '-' => {
                let number = read_number(&mut chars);
                let history_index = previous_history_index(history.len(), number)?;
                output.push_str(&history[history_index]);
            }
            value if value.is_ascii_digit() => {
                let number = read_number_after_first(value, &mut chars);
                if number == 0 || number > history.len() as i32 {
                    return Err(HistorySubstitutionError::EventNotFound(number));
                }

                output.push_str(&history[(number - 1) as usize]);
            }
            _ => {
                let prefix = read_prefix(selector, &mut chars, &options.separators);
                let Some(command) = history
                    .iter()
                    .rev()
                    .find(|command| command.starts_with(&prefix))
                else {
                    return Err(HistorySubstitutionError::PrefixNotFound(prefix));
                };

                output.push_str(command);
            }
        }

        changed = true;
    }

    if changed || internal_change {
        Ok(HistorySubstitutionResult {
            line: output,
            changed,
        })
    } else {
        Ok(HistorySubstitutionResult {
            line: line.to_owned(),
            changed: false,
        })
    }
}

pub fn complete_file_line(line: &str, request: FileCompletionRequest) -> FileCompletionResult {
    let word = completion_word(line, DEFAULT_SEPARATORS);
    let directory = expand_completion_directory(&word.directory_text);

    let Ok(entries) = read_directory_names(&directory) else {
        return FileCompletionResult {
            line: line.to_owned(),
            listing: Vec::new(),
            beep: true,
        };
    };

    complete_file_line_with_candidates(line, &word, request, entries)
}

pub fn complete_file_line_with_candidates<I, S>(
    line: &str,
    word: &CompletionWord,
    request: FileCompletionRequest,
    candidates: I,
) -> FileCompletionResult
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut matches = candidates
        .into_iter()
        .map(Into::into)
        .filter(|candidate| candidate.starts_with(&word.file_prefix))
        .collect::<Vec<_>>();

    match request {
        FileCompletionRequest::Complete => {
            matches.retain(|candidate| !word.file_prefix.is_empty() || !candidate.starts_with('.'));

            let Some(common) = common_completion_prefix(&matches, &word.file_prefix) else {
                return FileCompletionResult {
                    line: line.to_owned(),
                    listing: Vec::new(),
                    beep: true,
                };
            };

            if common == word.file_prefix {
                return FileCompletionResult {
                    line: line.to_owned(),
                    listing: Vec::new(),
                    beep: true,
                };
            }

            let mut completed = String::new();
            completed.push_str(&line[..word.prefix_start]);
            completed.push_str(&word.directory_text);
            completed.push_str(&common);

            FileCompletionResult {
                line: completed,
                listing: Vec::new(),
                beep: false,
            }
        }
        FileCompletionRequest::List => {
            matches.retain(|candidate| !word.file_prefix.is_empty() || !candidate.starts_with('.'));
            matches.sort();

            FileCompletionResult {
                line: line.to_owned(),
                listing: matches,
                beep: false,
            }
        }
    }
}

pub fn completion_word(line: &str, separators: &str) -> CompletionWord {
    let last_word_start = line
        .char_indices()
        .rev()
        .find(|(_, ch)| separators.contains(*ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);

    let last_word = &line[last_word_start..];
    let Some(slash_index) = last_word.rfind('/') else {
        return CompletionWord {
            prefix_start: last_word_start,
            directory_text: String::new(),
            file_prefix: last_word.to_owned(),
        };
    };

    let directory_end = last_word_start + slash_index + 1;

    CompletionWord {
        prefix_start: last_word_start,
        directory_text: line[last_word_start..directory_end].to_owned(),
        file_prefix: line[directory_end..].to_owned(),
    }
}

pub fn render_completion_listing(
    names: &[String],
    column_width: usize,
    terminal_width: usize,
) -> String {
    if names.is_empty() {
        return String::new();
    }

    let width = column_width.max(1);
    let mut output = String::new();
    let mut column = width;

    for name in names {
        output.push_str(&format!("{name:<width$}"));
        column += width;
        if column >= terminal_width {
            column = width;
            output.push('\n');
        }
    }

    if column != width {
        output.push('\n');
    }

    output
}

fn substitute_previous_command(
    line: &str,
    last: &str,
    substitute_char: char,
) -> Result<HistorySubstitutionResult, HistorySubstitutionError> {
    let Some(rest) = line.strip_prefix(substitute_char) else {
        return Ok(HistorySubstitutionResult {
            line: line.to_owned(),
            changed: false,
        });
    };

    let Some(separator) = rest.find(substitute_char) else {
        return Err(HistorySubstitutionError::ModifierFailed);
    };

    let old = &rest[..separator];
    let new = &rest[separator + substitute_char.len_utf8()..];
    let Some(start) = last.find(old) else {
        return Err(HistorySubstitutionError::ModifierFailed);
    };

    let mut output = String::new();
    output.push_str(&last[..start]);
    output.push_str(new);
    output.push_str(&last[start + old.len()..]);

    Ok(HistorySubstitutionResult {
        line: output,
        changed: true,
    })
}

fn read_number(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) -> i32 {
    let mut number = 0;

    while let Some((_, ch)) = chars.peek().copied() {
        if !ch.is_ascii_digit() {
            break;
        }

        chars.next();
        number *= 10;
        number += i32::from(ch as u8 - b'0');
    }

    number
}

fn read_number_after_first(
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> i32 {
    let mut number = i32::from(first as u8 - b'0');

    while let Some((_, ch)) = chars.peek().copied() {
        if !ch.is_ascii_digit() {
            break;
        }

        chars.next();
        number *= 10;
        number += i32::from(ch as u8 - b'0');
    }

    number
}

fn read_prefix(
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    separators: &str,
) -> String {
    let mut prefix = String::new();
    prefix.push(first);

    while let Some((_, ch)) = chars.peek().copied() {
        if separators.contains(ch) {
            break;
        }

        chars.next();
        prefix.push(ch);
    }

    prefix
}

fn previous_history_index(
    history_len: usize,
    previous: i32,
) -> Result<usize, HistorySubstitutionError> {
    if previous == 0 || previous > history_len as i32 {
        return Err(HistorySubstitutionError::EventNotFound(
            history_len as i32 - previous + 1,
        ));
    }

    Ok(history_len - previous as usize)
}

fn argument_selector(
    line: &str,
    selector: i32,
    separators: &str,
) -> Result<String, HistorySubstitutionError> {
    let words = split_words(line, separators);

    match selector {
        -1 => words.last().map(|value| (*value).to_owned()).ok_or(
            HistorySubstitutionError::BadArgumentSelector(DEFAULT_HISTORY_CHAR),
        ),
        -2 => {
            if words.len() <= 1 {
                Ok(String::new())
            } else {
                Ok(words[1..].join(" "))
            }
        }
        value if value >= 0 => words
            .get(value as usize)
            .map(|value| (*value).to_owned())
            .ok_or(HistorySubstitutionError::BadArgumentSelector(
                DEFAULT_HISTORY_CHAR,
            )),
        _ => Err(HistorySubstitutionError::BadArgumentSelector(
            DEFAULT_HISTORY_CHAR,
        )),
    }
}

fn split_words<'a>(line: &'a str, separators: &str) -> Vec<&'a str> {
    line.split(|ch| separators.contains(ch))
        .filter(|word| !word.is_empty())
        .collect()
}

fn common_completion_prefix(matches: &[String], actual: &str) -> Option<String> {
    let mut iter = matches.iter();
    let first = iter.next()?;
    let mut prefix = first.clone();

    for candidate in iter {
        prefix = common_prefix(&prefix, candidate);
        if prefix == actual {
            break;
        }
    }

    Some(prefix)
}

fn common_prefix(left: &str, right: &str) -> String {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| left == right)
        .map(|(ch, _)| ch)
        .collect()
}

fn expand_completion_directory(directory_text: &str) -> PathBuf {
    let directory = if directory_text.is_empty() {
        "."
    } else {
        directory_text.trim_end_matches('/')
    };

    if directory == "." {
        return PathBuf::from(directory);
    }

    if let Some(rest) = directory.strip_prefix("~/") {
        if let Some(home) = home_directory() {
            return Path::new(&home).join(rest);
        }
    }

    PathBuf::from(directory)
}

fn read_directory_names(directory: &Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }

    Ok(names)
}

fn home_directory() -> Option<String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|value| value.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history() -> Vec<String> {
        vec![
            "read first.blif".to_owned(),
            "collapse -v".to_owned(),
            "write_blif output.blif".to_owned(),
        ]
    }

    #[test]
    fn prompt_replaces_history_character_with_next_index() {
        assert_eq!(render_prompt(Some("sis[%]> "), 4, '%'), "sis[5]> ");
        assert_eq!(render_prompt(None, 4, '%'), "");
    }

    #[test]
    fn trims_initial_space_without_marking_substitution() {
        let result = substitute_history(
            "   print_stats",
            &history(),
            &HistorySubstitutionOptions::default(),
        )
        .unwrap();

        assert_eq!(result.line, "print_stats");
        assert!(!result.changed);
    }

    #[test]
    fn substitutes_last_command() {
        let result =
            substitute_history("%%", &history(), &HistorySubstitutionOptions::default()).unwrap();

        assert_eq!(result.line, "write_blif output.blif");
        assert!(result.changed);
    }

    #[test]
    fn substitutes_arguments_from_last_command() {
        let options = HistorySubstitutionOptions::default();

        assert_eq!(
            substitute_history("echo %$", &history(), &options)
                .unwrap()
                .line,
            "echo output.blif"
        );
        assert_eq!(
            substitute_history("echo %*", &history(), &options)
                .unwrap()
                .line,
            "echo output.blif"
        );
        assert_eq!(
            substitute_history("echo %:0 %:1", &history(), &options)
                .unwrap()
                .line,
            "echo write_blif output.blif"
        );
    }

    #[test]
    fn substitutes_absolute_previous_and_prefix_events() {
        let options = HistorySubstitutionOptions::default();

        assert_eq!(
            substitute_history("%1", &history(), &options).unwrap().line,
            "read first.blif"
        );
        assert_eq!(
            substitute_history("%-2", &history(), &options)
                .unwrap()
                .line,
            "collapse -v"
        );
        assert_eq!(
            substitute_history("%coll", &history(), &options)
                .unwrap()
                .line,
            "collapse -v"
        );
    }

    #[test]
    fn escaped_history_character_is_unescaped_without_history_lookup() {
        let result = substitute_history(
            r"echo \%literal",
            &[],
            &HistorySubstitutionOptions::default(),
        )
        .unwrap();

        assert_eq!(result.line, "echo %literal");
        assert!(!result.changed);
    }

    #[test]
    fn caret_substitution_replaces_previous_command_fragment() {
        let result = substitute_history(
            "^output^new",
            &history(),
            &HistorySubstitutionOptions::default(),
        )
        .unwrap();

        assert_eq!(result.line, "write_blif new.blif");
        assert!(result.changed);
    }

    #[test]
    fn reports_legacy_history_errors() {
        assert_eq!(
            substitute_history("%1", &[], &HistorySubstitutionOptions::default()).unwrap_err(),
            HistorySubstitutionError::EventNotFound(0)
        );
        assert_eq!(
            substitute_history(
                "%missing",
                &history(),
                &HistorySubstitutionOptions::default()
            )
            .unwrap_err(),
            HistorySubstitutionError::PrefixNotFound("missing".to_owned())
        );
        assert_eq!(
            substitute_history("%:9", &history(), &HistorySubstitutionOptions::default())
                .unwrap_err(),
            HistorySubstitutionError::BadArgumentSelector('%')
        );
    }

    #[test]
    fn finds_completion_word_after_separators_and_directory() {
        assert_eq!(
            completion_word("read ~/pla/ex", DEFAULT_SEPARATORS),
            CompletionWord {
                prefix_start: 5,
                directory_text: "~/pla/".to_owned(),
                file_prefix: "ex".to_owned(),
            }
        );
        assert_eq!(
            completion_word("read ex", DEFAULT_SEPARATORS),
            CompletionWord {
                prefix_start: 5,
                directory_text: String::new(),
                file_prefix: "ex".to_owned(),
            }
        );
    }

    #[test]
    fn completes_to_common_prefix() {
        let word = completion_word("read al", DEFAULT_SEPARATORS);
        let result = complete_file_line_with_candidates(
            "read al",
            &word,
            FileCompletionRequest::Complete,
            ["alpha.blif", "alpine.eqn", "beta.blif"],
        );

        assert_eq!(result.line, "read alp");
        assert!(!result.beep);
    }

    #[test]
    fn completion_beeps_when_no_progress_is_available() {
        let word = completion_word("read alp", DEFAULT_SEPARATORS);
        let result = complete_file_line_with_candidates(
            "read alp",
            &word,
            FileCompletionRequest::Complete,
            ["alpha.blif", "alpine.eqn"],
        );

        assert_eq!(result.line, "read alp");
        assert!(result.beep);
    }

    #[test]
    fn listing_sorts_matches_and_hides_dot_files_for_empty_prefix() {
        let word = completion_word("read ", DEFAULT_SEPARATORS);
        let result = complete_file_line_with_candidates(
            "read ",
            &word,
            FileCompletionRequest::List,
            ["zeta", ".hidden", "alpha"],
        );

        assert_eq!(result.listing, ["alpha".to_owned(), "zeta".to_owned()]);
        assert_eq!(result.line, "read ");
    }

    #[test]
    fn renders_completion_listing_in_fixed_width_columns() {
        let names = vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()];

        assert_eq!(
            render_completion_listing(&names, 8, 16),
            "alpha   \nbeta    \ngamma   \n"
        );
    }

    #[test]
    fn source_contains_no_dependency_tracking_metadata_or_c_abi_exports() {
        let source = include_str!("filec.rs");

        assert!(!source.contains(concat!("REQUIRED", "_")));
        assert!(!source.contains(concat!("Port", "Dependency")));
        assert!(!source.contains(concat!("bead", "_", "id")));
        assert!(!source.contains(concat!("source", "_", "file")));
        assert!(!source.contains(concat!("LogicFriday1", "-", "8j8")));
        assert!(!source.contains(concat!("no", "_", "mangle")));
        assert!(!source.contains("extern \"C\""));
    }
}
