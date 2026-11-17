//! The places that aliases come from (ADR-0228).

use std::path::PathBuf;

use toml::de::{DeTable, DeValue};

use crate::entry::Item;

/// Whether the aliases of a source differ between machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The aliases belong to a person, such as the entries of their aliases
    /// file. They differ between machines, so the root's help leaves them
    /// out unless the tool opts out of `Rule::PersonalAliases`, and
    /// crumbkit-test's checks leave them out.
    Personal,
    /// The aliases ship with the tool or its repository, so they are the
    /// same on every machine. The root's help names them.
    Shipped,
}

/// A table of keys and the words that each stands for, as
/// [`Source::table`] takes it.
pub type Table = &'static [(&'static str, &'static [&'static str])];

/// One entry of a source: the words a person types after the tool's name,
/// and the words that replace them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The key and the replacement words.
    item: Item,
}

impl Entry {
    /// Returns the entry that replaces `key`, one or more words separated
    /// by spaces such as `wt ls`, with `words`.
    ///
    /// An entry whose key has no word, or whose `words` are empty, does not
    /// expand, and `aliases list` shows its problem.
    #[must_use]
    pub fn new<I, W>(key: &str, words: I) -> Self
    where
        I: IntoIterator<Item = W>,
        W: Into<String>,
    {
        let words: Vec<String> = words.into_iter().map(Into::into).collect();
        Self {
            item: Item::new(key, Some(words)),
        }
    }
}

/// Where a source's entries come from.
#[derive(Clone, Debug)]
enum Origin {
    /// An aliases file, or `None` when the environment has no config
    /// directory.
    File(Option<PathBuf>),
    /// A table in the tool's code.
    Table(&'static str, Table),
    /// A function that returns the entries when a run needs them.
    Lazy(&'static str, fn() -> Result<Vec<Entry>, String>),
    /// A file's text, for the tests.
    #[cfg(test)]
    Text(String, Option<String>),
}

/// An ordered place that aliases come from: a file, a table in the tool's
/// code or a function that loads them.
///
/// A key is one or more words, such as `"wt ls"`, and its value is one or
/// more words. An alias matches the leading words after the tool's name,
/// and the longest key in a source wins.
#[derive(Clone, Debug)]
pub struct Source {
    /// Where the entries come from.
    origin: Origin,
    /// Whether the entries differ between machines.
    kind: Kind,
    /// Whether the root's help names the entries.
    listed: bool,
}

/// What a source holds when a run reads it.
#[derive(Debug)]
pub(crate) enum Loaded {
    /// No file is at the path.
    Missing,
    /// The entries of the source.
    Items(Vec<Item>),
    /// The source cannot be read or parsed, for this reason.
    Unreadable(String),
}

impl Source {
    /// Returns the personal source that reads the aliases file at `path`,
    /// which [`file`](crate::file) returns, or `None` when the environment
    /// has no config directory.
    ///
    /// A key of the file is one or more words, and its value is a list of
    /// one or more words, as in `"wt ls" = ["books", "list"]`. `expand` reads
    /// the file for every command line, so a key can start with a command,
    /// such as `books recent`, when the key as a whole is no command.
    #[must_use]
    pub const fn file(path: Option<PathBuf>) -> Self {
        Self {
            origin: Origin::File(path),
            kind: Kind::Personal,
            listed: true,
        }
    }

    /// Returns the shipped source that holds `entries` in the tool's code.
    /// `origin` says where the entries come from, and `aliases list` prints
    /// it in each row's `from` column.
    ///
    /// `expand` consults a table for every command line, and a key can
    /// start with a command, as in `books recent`, when the key as a whole is
    /// no command.
    #[must_use]
    pub const fn table(origin: &'static str, entries: Table) -> Self {
        Self {
            origin: Origin::Table(origin, entries),
            kind: Kind::Shipped,
            listed: true,
        }
    }

    /// Returns the source that calls `read` for its entries when a run needs
    /// them. `origin` says where the entries come from.
    ///
    /// `expand` calls `read` only when the first word after the tool's name
    /// is no command, so a run that types a command pays nothing. A key of
    /// more than one word cannot start with a command. A failure
    /// of `read` skips the source, and `aliases list` shows its message in
    /// one row.
    #[must_use]
    pub const fn lazy(
        origin: &'static str,
        kind: Kind,
        read: fn() -> Result<Vec<Entry>, String>,
    ) -> Self {
        Self {
            origin: Origin::Lazy(origin, read),
            kind,
            listed: true,
        }
    }

    /// Returns this source without its entries on the root's help, for
    /// shortcuts that a tool keeps out of the way.
    #[must_use]
    pub const fn unlisted(mut self) -> Self {
        self.listed = false;
        self
    }

    /// Returns a personal source that holds `text` as an aliases file at
    /// `origin`, or no file when `text` is `None`.
    #[cfg(test)]
    pub(crate) fn text_at(origin: &str, kind: Kind, text: Option<&str>) -> Self {
        Self {
            origin: Origin::Text(origin.to_owned(), text.map(str::to_owned)),
            kind,
            listed: true,
        }
    }

    /// Returns a source that holds `text` as an aliases file, or no file
    /// when `text` is `None`.
    #[cfg(test)]
    pub(crate) fn text(kind: Kind, text: Option<&str>) -> Self {
        Self::text_at("aliases file", kind, text)
    }

    /// Returns whether the source's entries differ between machines.
    pub(crate) const fn kind(&self) -> Kind {
        self.kind
    }

    /// Returns whether the root's help names the source's entries.
    pub(crate) const fn listed(&self) -> bool {
        self.listed
    }

    /// Returns whether the source calls a function to load, so `expand`
    /// leaves it alone when the first word is a command.
    pub(crate) const fn deferred(&self) -> bool {
        matches!(self.origin, Origin::Lazy(..))
    }

    /// Returns the table of a [`Source::table`], which `expand` matches
    /// without copying it, or `None` for any other source.
    pub(crate) const fn table_entries(&self) -> Option<Table> {
        match self.origin {
            Origin::Table(_, entries) => Some(entries),
            _ => None,
        }
    }

    /// Returns the label that `aliases list` prints for the source: the
    /// path of a file, or the origin that the tool gave.
    pub(crate) fn origin(&self) -> String {
        match &self.origin {
            Origin::File(path) => path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            Origin::Table(origin, _) | Origin::Lazy(origin, _) => (*origin).to_owned(),
            #[cfg(test)]
            Origin::Text(origin, _) => origin.clone(),
        }
    }

    /// Returns the file that the source reads and whether it exists, for the
    /// note of `aliases list`, or `None` for a source that is not a file.
    /// The inner `Option` is the path, which is `None` when the environment
    /// has no config directory.
    pub(crate) fn file_state(&self) -> Option<(Option<String>, bool)> {
        match &self.origin {
            Origin::File(path) => {
                let exists = path.as_ref().is_some_and(|path| path.exists());
                Some((path.as_ref().map(|path| path.display().to_string()), exists))
            }
            #[cfg(test)]
            Origin::Text(origin, text) => Some((Some(origin.clone()), text.is_some())),
            Origin::Table(..) | Origin::Lazy(..) => None,
        }
    }

    /// Reads the source.
    pub(crate) fn load(&self) -> Loaded {
        match &self.origin {
            Origin::File(None) => Loaded::Missing,
            Origin::File(Some(path)) => match crate::file::read(path) {
                Ok(Some(text)) => parse(&text),
                Ok(None) => Loaded::Missing,
                Err(error) => Loaded::Unreadable(error.to_string()),
            },
            Origin::Table(_, entries) => Loaded::Items(
                entries
                    .iter()
                    .map(|(key, words)| {
                        let words = words.iter().map(|&word| word.to_owned()).collect();
                        Item::new(key, Some(words))
                    })
                    .collect(),
            ),
            Origin::Lazy(_, read) => match read() {
                Ok(entries) => Loaded::Items(entries.into_iter().map(|entry| entry.item).collect()),
                Err(reason) => Loaded::Unreadable(reason),
            },
            #[cfg(test)]
            Origin::Text(_, text) => text.as_deref().map_or(Loaded::Missing, parse),
        }
    }
}

/// Parses the text of an aliases file into its items.
fn parse(text: &str) -> Loaded {
    match DeTable::parse(text) {
        Ok(table) => Loaded::Items(
            table
                .get_ref()
                .iter()
                .map(|(key, value)| Item::new(key.get_ref(), words(value.get_ref())))
                .collect(),
        ),
        Err(error) => Loaded::Unreadable(parse_reason(text, &error)),
    }
}

/// Returns the words of an entry's value, or `None` when the value is not a
/// list of one or more strings.
fn words(value: &DeValue<'_>) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|word| word.get_ref().as_str().map(str::to_owned))
        .collect()
}

/// Returns the reason that `text` is not a TOML table: the parser's message,
/// after the line number of the place it points at when it points at one.
fn parse_reason(text: &str, error: &toml::de::Error) -> String {
    let line = error
        .span()
        .and_then(|span| text.get(..span.start))
        .map(|before| before.matches('\n').count() + 1);
    match line {
        Some(line) => format!("line {line}: {}", error.message()),
        None => error.message().to_owned(),
    }
}
