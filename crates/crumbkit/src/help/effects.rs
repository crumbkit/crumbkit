//! A command's effects, in the vocabulary of the Model Context Protocol's
//! tool annotations: read-only, destructive, idempotent and open-world, and
//! the write guards that a command declares beside them.

use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use serde_json::{Value, json};

#[cfg(test)]
mod tests;

/// What a command does to the world it runs in, which a [`Leaf`] declares
/// with [`Leaf::effects`](super::Leaf::effects).
///
/// The properties are those of the Model Context Protocol's tool
/// annotations (`readOnlyHint`, `destructiveHint`, `idempotentHint` and
/// `openWorldHint`), so an agent that knows one reads the other. Each
/// constructor sets what the command changes, and two builders add the
/// other properties:
///
/// ```
/// use crumbkit::help::Effects;
///
/// const LIST: Effects = Effects::read_only();
/// const REMOVE: Effects = Effects::destructive().idempotent();
///
/// assert!(LIST.is_read_only());
/// assert!(REMOVE.is_destructive());
/// ```
///
/// A command's `-h` and `--help` print the effects on one line after its
/// examples, such as `Effects: destructive, idempotent, closed-world.`, and
/// `commands list` writes them as an object under `effects`.
///
/// A command that writes can also declare its write guards (ADR-0047).
/// [`Effects::replacing`] declares a command that replaces existing data,
/// which takes [`Expect`](crate::Expect) so a caller can make it fail when
/// the data changed since the caller read it. [`Effects::all_or_none`] and
/// [`Effects::per_item`] declare how a write of many items ends when one of
/// them fails:
///
/// ```
/// use crumbkit::help::{Effects, Items};
///
/// const EDIT: Effects = Effects::replacing();
/// const REMOVE: Effects = Effects::destructive().idempotent().all_or_none();
///
/// assert!(EDIT.is_replacing() && EDIT.is_destructive());
/// assert_eq!(REMOVE.items(), Some(Items::AllOrNone));
/// ```
///
/// The help of such a command prints a `Writes:` line after the effects
/// line, such as `Writes: replaces existing data; --expect <VERSION> fails
/// with a conflict when the data changed.`, and `commands list` writes the
/// guards as an object under `writes`.
///
/// [`Leaf`]: super::Leaf
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effects {
    /// What the command changes.
    change: Change,
    /// Whether a second run with the same arguments changes nothing more.
    idempotent: bool,
    /// Whether the command reaches systems outside the tool, such as a web
    /// service.
    open_world: bool,
    /// Whether the command replaces existing data, which
    /// [`Effects::replacing`] sets.
    replaces: bool,
    /// How a write of many items ends, or `None` when the tool declares
    /// nothing.
    items: Option<Items>,
}

/// How a command that writes many items ends when one of them fails, which
/// [`Effects::all_or_none`] and [`Effects::per_item`] declare (ADR-0047).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Items {
    /// The command writes every item or none of them.
    AllOrNone,
    /// The command writes each item that it can, reports a result for each
    /// item, and exits with code 1 when any item fails.
    PerItem,
}

impl Items {
    /// Returns the name that `commands list` writes under `items`, such as
    /// `all_or_none`.
    const fn name(self) -> &'static str {
        match self {
            Self::AllOrNone => "all_or_none",
            Self::PerItem => "per_item",
        }
    }
}

/// What a command changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Change {
    /// Nothing.
    Nothing,
    /// Something, and it deletes or overwrites nothing.
    Additive,
    /// Something, and it may delete or overwrite data.
    Destructive,
}

impl Effects {
    /// Returns the effects of a command that changes nothing. Such a command
    /// is idempotent, because a repeat changes nothing either.
    #[must_use]
    pub const fn read_only() -> Self {
        Self {
            change: Change::Nothing,
            idempotent: true,
            open_world: false,
            replaces: false,
            items: None,
        }
    }

    /// Returns the effects of a command that changes something and deletes
    /// or overwrites nothing, such as one that adds a record.
    #[must_use]
    pub const fn additive() -> Self {
        Self {
            change: Change::Additive,
            idempotent: false,
            open_world: false,
            replaces: false,
            items: None,
        }
    }

    /// Returns the effects of a command that may delete or overwrite data.
    /// crumbkit-test's `check_effects` requires `--yes` on such a command.
    #[must_use]
    pub const fn destructive() -> Self {
        Self {
            change: Change::Destructive,
            idempotent: false,
            open_world: false,
            replaces: false,
            items: None,
        }
    }

    /// Returns the effects of a command that replaces existing data, such as
    /// one that sets a record to a new value. Replacing overwrites data, so
    /// the command is destructive, and crumbkit-test's `check_effects`
    /// requires `--yes` on it. The check also requires crumbkit's
    /// [`Expect`](crate::Expect), which adds `--expect <VERSION>`, so that
    /// two concurrent writers cannot lose each other's changes unnoticed.
    #[must_use]
    pub const fn replacing() -> Self {
        Self {
            change: Change::Destructive,
            idempotent: false,
            open_world: false,
            replaces: true,
            items: None,
        }
    }

    /// Declares that the command, which writes many items, writes every
    /// item or none of them, so a caller that sees a failure knows that
    /// nothing changed. crumbkit-test's `check_effects` requires this or
    /// [`Effects::per_item`] on a write that takes many values in a
    /// positional argument.
    #[must_use]
    pub const fn all_or_none(mut self) -> Self {
        self.items = Some(Items::AllOrNone);
        self
    }

    /// Declares that the command, which writes many items, writes each item
    /// that it can and reports a result for each, and exits with code 1
    /// when any item fails. Such a command prints a
    /// [`Batch`](crate::Batch) or a [`Prose`](crate::Prose), whose
    /// `failures` name the items that failed.
    #[must_use]
    pub const fn per_item(mut self) -> Self {
        self.items = Some(Items::PerItem);
        self
    }

    /// Marks the command as idempotent: a second run with the same
    /// arguments changes nothing more than the first. A read-only command
    /// is already idempotent.
    #[must_use]
    pub const fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    /// Marks the command as reaching systems outside the tool, such as a
    /// web service, whose state the tool does not control. A command
    /// without it works only on the tool's own data.
    #[must_use]
    pub const fn open_world(mut self) -> Self {
        self.open_world = true;
        self
    }

    /// Reports whether the command changes nothing.
    #[must_use]
    pub const fn is_read_only(&self) -> bool {
        matches!(self.change, Change::Nothing)
    }

    /// Reports whether the command may delete or overwrite data.
    #[must_use]
    pub const fn is_destructive(&self) -> bool {
        matches!(self.change, Change::Destructive)
    }

    /// Reports whether the command replaces existing data, as
    /// [`Effects::replacing`] declares.
    #[must_use]
    pub const fn is_replacing(&self) -> bool {
        self.replaces
    }

    /// Returns how the command ends when one of many items fails, or `None`
    /// when the tool declares nothing.
    #[must_use]
    pub const fn items(&self) -> Option<Items> {
        self.items
    }

    /// Reports whether a second run with the same arguments changes nothing
    /// more than the first. Only the tests read it, because help and
    /// `commands list` read the field.
    #[cfg(test)]
    pub(crate) const fn is_idempotent(self) -> bool {
        self.idempotent
    }

    /// Reports whether the command reaches systems outside the tool. Only
    /// the tests read it, because help and `commands list` read the field.
    #[cfg(test)]
    pub(crate) const fn is_open_world(self) -> bool {
        self.open_world
    }

    /// Returns the line that a leaf's `-h` and `--help` print after its
    /// examples, in help's wording, such as `Effects: read-only,
    /// closed-world.` A command that changes something names whether it is
    /// idempotent, so a missing word never stands for "no". Another
    /// rendering of help, such as a Markdown reference, prints this line so
    /// that it words the effects as help does.
    ///
    /// ```
    /// use crumbkit::help::Effects;
    ///
    /// assert_eq!(
    ///     Effects::destructive().idempotent().line(),
    ///     "Effects: destructive, idempotent, closed-world."
    /// );
    /// ```
    #[must_use]
    pub const fn line(self) -> &'static str {
        match (self.change, self.idempotent, self.open_world) {
            (Change::Nothing, _, false) => "Effects: read-only, closed-world.",
            (Change::Nothing, _, true) => "Effects: read-only, open-world.",
            (Change::Additive, false, false) => "Effects: additive, not idempotent, closed-world.",
            (Change::Additive, false, true) => "Effects: additive, not idempotent, open-world.",
            (Change::Additive, true, false) => "Effects: additive, idempotent, closed-world.",
            (Change::Additive, true, true) => "Effects: additive, idempotent, open-world.",
            (Change::Destructive, false, false) => {
                "Effects: destructive, not idempotent, closed-world."
            }
            (Change::Destructive, false, true) => {
                "Effects: destructive, not idempotent, open-world."
            }
            (Change::Destructive, true, false) => "Effects: destructive, idempotent, closed-world.",
            (Change::Destructive, true, true) => "Effects: destructive, idempotent, open-world.",
        }
    }
}

impl Effects {
    /// Returns the line that a leaf's `-h` and `--help` print after its
    /// effects line, which names how the command guards what it writes, or
    /// `None` for a read-only command and for a write that declares no
    /// guard. `expect` says whether the command takes crumbkit's
    /// `--expect`, which the command tree holds. Another rendering of help,
    /// such as a Markdown reference, prints this line so that it words the
    /// guards as help does.
    ///
    /// ```
    /// use crumbkit::help::Effects;
    ///
    /// assert_eq!(
    ///     Effects::destructive().all_or_none().writes_line(false).as_deref(),
    ///     Some("Writes: applies all items or none.")
    /// );
    /// ```
    #[must_use]
    pub fn writes_line(self, expect: bool) -> Option<String> {
        if self.is_read_only() {
            return None;
        }
        let mut clauses = Vec::new();
        match (self.replaces, expect) {
            (true, false) => clauses.push("replaces existing data, without a version check"),
            (true, true) => clauses.push("replaces existing data"),
            (false, _) => {}
        }
        if expect {
            clauses.push("--expect <VERSION> fails with a conflict when the data changed");
        }
        match self.items {
            Some(Items::AllOrNone) => clauses.push("applies all items or none"),
            Some(Items::PerItem) => {
                clauses.push("reports each item and exits 1 when any item fails");
            }
            None => {}
        }
        if clauses.is_empty() {
            return None;
        }
        Some(format!("Writes: {}.", clauses.join("; ")))
    }
}

/// How a command guards what it writes, as `commands list` writes it under
/// `writes`: whether it replaces data, whether it takes crumbkit's
/// `--expect`, and how a write of many items ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Writes {
    /// Whether the command replaces existing data.
    replaces: bool,
    /// Whether the command takes crumbkit's `--expect`.
    expect: bool,
    /// How a write of many items ends.
    items: Option<Items>,
}

/// Returns the guards of a command with `effects` that takes crumbkit's
/// `--expect` when `expect` is set, or `None` when the command declares no
/// effects or is read-only.
pub(crate) fn writes(effects: Option<Effects>, expect: bool) -> Option<Writes> {
    let effects = effects.filter(|effects| !effects.is_read_only())?;
    Some(Writes {
        replaces: effects.replaces,
        expect,
        items: effects.items,
    })
}

/// Serializes the guards as an object, such as
/// `{"replaces":true,"expect":true,"items":null}`.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`.
impl Serialize for Writes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut writes = serializer.serialize_struct("Writes", 3)?;
        writes.serialize_field("replaces", &self.replaces)?;
        writes.serialize_field("expect", &self.expect)?;
        writes.serialize_field("items", &self.items.map(Items::name))?;
        writes.end()
    }
}

/// Returns the JSON Schema of the object that `Writes` serializes, or
/// `null` for a command that is read-only or declares no effects, which
/// `commands list` describes under `writes`.
pub(crate) fn writes_schema() -> Value {
    json!({
        "description": "How the command guards what it writes, or null when the command is read-only or the tool declares no effects.",
        "type": ["object", "null"],
        "properties": {
            "replaces": {
                "description": "Whether the command replaces existing data.",
                "type": "boolean",
            },
            "expect": {
                "description": "Whether the command takes `--expect <VERSION>`, which fails with a conflict unless the target's current version is VERSION, or `none` when the target must not exist.",
                "type": "boolean",
            },
            "items": {
                "description": "all_or_none when the command writes every item or none, per_item when it reports each item and exits 1 when any item fails, and null when the tool declares neither.",
                "enum": ["all_or_none", "per_item", null],
            },
        },
        "required": ["replaces", "expect", "items"],
    })
}

/// Serializes the effects as an object with the four properties of the
/// Model Context Protocol's tool annotations, in its order, such as
/// `{"read_only":true,"destructive":false,"idempotent":true,"open_world":false}`.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`.
impl Serialize for Effects {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut effects = serializer.serialize_struct("Effects", 4)?;
        effects.serialize_field("read_only", &self.is_read_only())?;
        effects.serialize_field("destructive", &self.is_destructive())?;
        effects.serialize_field("idempotent", &self.idempotent)?;
        effects.serialize_field("open_world", &self.open_world)?;
        effects.end()
    }
}

/// Returns the JSON Schema of the object that `Serialize` writes, or `null`
/// for a command that declares no effects, which `commands list` describes
/// under `effects`.
pub(crate) fn schema() -> Value {
    json!({
        "description": "The command's effects, as the Model Context Protocol's tool annotations name them, or null when the tool declares none, in which case a caller assumes the command may destroy data.",
        "type": ["object", "null"],
        "properties": {
            "read_only": {
                "description": "Whether the command changes nothing.",
                "type": "boolean",
            },
            "destructive": {
                "description": "Whether the command may delete or overwrite data.",
                "type": "boolean",
            },
            "idempotent": {
                "description": "Whether a second run with the same arguments changes nothing more than the first.",
                "type": "boolean",
            },
            "open_world": {
                "description": "Whether the command reaches systems outside the tool, such as a web service.",
                "type": "boolean",
            },
        },
        "required": ["read_only", "destructive", "idempotent", "open_world"],
    })
}
