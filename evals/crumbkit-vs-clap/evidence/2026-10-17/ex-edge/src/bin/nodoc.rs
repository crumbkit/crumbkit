use clap::Parser;
use crumbkit::{Output, Page};
#[derive(Parser)]
#[command(name = "nodoc")]
struct Cli {
    #[command(flatten)]
    output: Output,
    #[command(flatten)]
    page: Page<2>,
}
fn main() { let _ = Cli::parse(); }
