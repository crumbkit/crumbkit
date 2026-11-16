//! Replacing an alias on the command line with its words.

use std::ffi::OsString;

use clap::Command;

use crate::Aliases;
use crate::entry::{longest, longest_in, wins};
use crate::source::Loaded;

#[cfg(test)]
mod tests;

impl Aliases {
    /// Returns the process arguments `args` with an alias replaced by its
    /// words, for the tool to pass to `crumbkit::try_parse_to`.
    ///
    /// `command` is the tool's command tree before parsing. The words after
    /// the tool's name are matched against the keys of each source in
    /// order. A key is one or more words, and it matches when it spells the
    /// leading words. The longest matching key in a source wins, and the
    /// first source with a match wins. The key's words are replaced by the
    /// entry's words, and any words typed after the key follow them. The
    /// words are not expanded again, so an alias whose first word is another
    /// alias expands once, and no shell sees them.
    ///
    /// `expand` returns `args` unchanged when there is no first word, when
    /// the first word is not UTF-8 or starts with `-`, and when it is
    /// `help`. When the first word is a subcommand of the root or one of its
    /// aliases, hidden or not, `expand` leaves a lazy source unread and
    /// consults every other source. A real command always
    /// wins over an alias: a key that names a real command path, such as
    /// `books list`, never expands.
    ///
    /// A source that cannot be read or parsed is skipped, and the later
    /// sources still apply. `expand` never prints and never fails, because a
    /// broken source must not stop the tool's real commands or add text where
    /// an agent reads. `<tool> aliases list` reports each problem instead.
    ///
    /// When `expand` replaces an alias, it calls `crumbkit::events::aliased`,
    /// so the tool's run observer, such as crumbkit's usage log, learns that
    /// an alias expanded. The call does nothing in a tool without an
    /// observer.
    ///
    /// [`Source::table`]: crate::Source::table
    #[must_use]
    pub fn expand<I, T>(&self, command: &Command, args: I) -> Vec<OsString>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
        let Some(first) = args.get(1).and_then(|word| word.to_str()) else {
            return args;
        };
        if first.starts_with('-') || first == "help" {
            return args;
        }
        let first_is_command = wins(command, first);
        let typed: Vec<&str> = args
            .iter()
            .skip(1)
            .map_while(|word| word.to_str())
            .collect();
        for source in self.sources() {
            if source.deferred() && first_is_command {
                continue;
            }
            let (key_len, replacement): (usize, Vec<OsString>) =
                if let Some(table) = source.table_entries() {
                    let Some((len, words)) = longest_in(table, command, &typed) else {
                        continue;
                    };
                    (len, words.iter().map(OsString::from).collect())
                } else {
                    let Loaded::Items(items) = source.load() else {
                        continue;
                    };
                    let Some(item) = longest(&items, command, source.deferred(), &typed) else {
                        continue;
                    };
                    let Some(words) = &item.words else {
                        continue;
                    };
                    (item.key.len(), words.iter().map(OsString::from).collect())
                };
            // The run observer learns that an alias expanded, and the usage
            // log's line holds the expanded command (ADR-0029, ADR-0057).
            crumbkit::events::aliased();
            let mut rest = args.into_iter();
            return rest
                .next()
                .into_iter()
                .chain(replacement)
                .chain(rest.skip(key_len))
                .collect();
        }
        args
    }
}
