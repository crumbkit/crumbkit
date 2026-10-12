//! A command's effects, in the vocabulary of the Model Context Protocol's
//! tool annotations: read-only, destructive, idempotent and open-world.

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
/// assert!(LIST.is_idempotent());
/// assert!(REMOVE.is_destructive());
/// ```
///
/// A command's `-h` and `--help` print the effects on one line after its
/// examples, such as `Effects: destructive, idempotent, closed-world.`, and
/// `commands list` writes them as an object under `effects`.
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
        }
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

    /// Reports whether a second run with the same arguments changes nothing
    /// more than the first.
    #[must_use]
    pub const fn is_idempotent(&self) -> bool {
        self.idempotent
    }

    /// Reports whether the command reaches systems outside the tool.
    #[must_use]
    pub const fn is_open_world(&self) -> bool {
        self.open_world
    }

    /// Returns the line that help prints, such as `Effects: read-only,
    /// closed-world.` A command that changes something names whether it is
    /// idempotent, so a missing word never stands for "no".
    pub(crate) fn line(self) -> String {
        let world = if self.open_world {
            "open-world"
        } else {
            "closed-world"
        };
        let repeat = if self.idempotent {
            "idempotent"
        } else {
            "not idempotent"
        };
        match self.change {
            Change::Nothing => format!("Effects: read-only, {world}."),
            Change::Additive => format!("Effects: additive, {repeat}, {world}."),
            Change::Destructive => format!("Effects: destructive, {repeat}, {world}."),
        }
    }
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
