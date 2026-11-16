//! The verdict on every entry of every source, which `names` and `list`
//! share so they agree with `expand` on which entries work.

use clap::Command;

use crate::Aliases;
use crate::entry::{Item, wins};
use crate::source::{Loaded, Source};

/// An item of a source and why it does not expand, or `None` when it does.
pub(crate) struct Judged {
    /// The item.
    pub(crate) item: Item,
    /// The problem, which is empty when the item expands.
    pub(crate) problem: Option<String>,
}

/// What `expand` finds when it reads a source.
pub(crate) enum Content {
    /// No file is at the path.
    Missing,
    /// The source cannot be read or parsed, for this reason.
    Unreadable(String),
    /// The entries of the source, each with its verdict.
    Items(Vec<Judged>),
}

impl Aliases {
    /// Reads every source and returns it with the verdict on each entry. An
    /// entry that works in its own source still has a problem when an earlier
    /// source holds a working entry whose key is the whole of its key or the
    /// start of it, because `expand` takes the earlier entry first.
    pub(crate) fn judge(&self, command: &Command) -> Vec<(&Source, Content)> {
        let sources = self.sources();
        let loaded: Vec<Loaded> = sources.iter().map(Source::load).collect();
        let mut judged = Vec::with_capacity(sources.len());
        for (at, source) in sources.iter().enumerate() {
            let content = match &loaded[at] {
                Loaded::Missing => Content::Missing,
                Loaded::Unreadable(reason) => Content::Unreadable(reason.clone()),
                Loaded::Items(items) => Content::Items(
                    items
                        .iter()
                        .map(|item| Judged {
                            item: item.clone(),
                            problem: item
                                .problem(command, source.deferred())
                                .map(str::to_owned)
                                .or_else(|| {
                                    hidden_by(command, &sources[..at], &loaded[..at], item)
                                }),
                        })
                        .collect(),
                ),
            };
            judged.push((source, content));
        }
        judged
    }
}

/// Returns the problem of `item` when an entry of an earlier source in
/// `before`, which `loaded` holds, wins over it.
fn hidden_by(
    command: &Command,
    before: &[Source],
    loaded: &[Loaded],
    item: &Item,
) -> Option<String> {
    let first_is_command = wins(command, item.key.first()?);
    before.iter().zip(loaded).find_map(|(source, loaded)| {
        if source.deferred() && first_is_command {
            return None;
        }
        let Loaded::Items(items) = loaded else {
            return None;
        };
        items
            .iter()
            .any(|earlier| {
                earlier.key.len() <= item.key.len()
                    && earlier.key == item.key[..earlier.key.len()]
                    && earlier.problem(command, source.deferred()).is_none()
            })
            .then(|| format!("an entry of {} wins", source.origin()))
    })
}
