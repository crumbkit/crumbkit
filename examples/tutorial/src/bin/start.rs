//! The tutorial's starting point: a plain clap tool that lists the jars in a
//! pantry, before it adopts crumbkit.

use clap::{Args, Parser, Subcommand};

/// Keep track of the jars in a pantry.
#[derive(Parser)]
#[command(name = "pantry")]
struct Cli {
    /// The resource the command acts on.
    #[command(subcommand)]
    resource: Resource,
}

/// The resources that pantry serves.
#[derive(Subcommand)]
enum Resource {
    /// Work with the jars in the pantry.
    Jars {
        /// What to do with the jars.
        #[command(subcommand)]
        command: JarsCommand,
    },
}

/// The commands for jars.
#[derive(Subcommand)]
enum JarsCommand {
    /// List the jars, in id order.
    #[command(visible_alias = "ls")]
    List(JarsList),
}

/// The arguments of `pantry jars list`.
#[derive(Args)]
struct JarsList {
    /// List only the jars on this shelf
    #[arg(long)]
    shelf: Option<String>,
}

/// One jar in the pantry.
struct Jar {
    /// The jar's id.
    id: u32,
    /// What the jar holds.
    name: &'static str,
    /// The shelf the jar stands on.
    shelf: &'static str,
}

/// The jars in the pantry, in id order.
fn jars() -> Vec<Jar> {
    vec![
        Jar {
            id: 1,
            name: "rice",
            shelf: "top",
        },
        Jar {
            id: 2,
            name: "lentils",
            shelf: "top",
        },
        Jar {
            id: 3,
            name: "oats",
            shelf: "bottom",
        },
    ]
}

#[expect(
    clippy::print_stdout,
    reason = "the plain clap tool prints with println!, as a tool does before it adopts crumbkit"
)]
fn main() {
    let cli = Cli::parse();
    match cli.resource {
        Resource::Jars {
            command: JarsCommand::List(args),
        } => {
            for jar in jars() {
                if args.shelf.as_deref().is_none_or(|shelf| jar.shelf == shelf) {
                    println!("{}\t{}\t{}", jar.id, jar.name, jar.shelf);
                }
            }
        }
    }
}
