use std::process::ExitCode;
use clap::Parser;
use serde::Serialize;

#[derive(Serialize)]
struct Book { id: u32, title: &'static str, author: &'static str, year: u16 }

/// List the books in a small catalogue.
#[derive(Parser)]
#[command(name = "books")]
struct Cli {
    /// List only the books by this author
    #[arg(long)]
    author: Option<String>,
    /// Print JSON
    #[arg(long)]
    json: bool,
    /// Show every column
    #[arg(long)]
    long: bool,
    /// Rows per page
    #[arg(long, default_value_t = 2)]
    limit: usize,
    /// First row
    #[arg(long, default_value_t = 0)]
    offset: usize,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let books = vec![
        Book { id: 1, title: "Pride and Prejudice", author: "Jane Austen", year: 1813 },
        Book { id: 2, title: "Frankenstein", author: "Mary Shelley", year: 1818 },
        Book { id: 3, title: "Moby-Dick", author: "Herman Melville", year: 1851 },
    ];
    let books: Vec<Book> = books.into_iter()
        .filter(|b| cli.author.as_deref().is_none_or(|a| b.author == a)).collect();
    if let (Some(a), true) = (&cli.author, books.is_empty()) {
        eprintln!("error: the catalogue has no books by {a} [shelf:books:unknown_author]\nNext: books");
        return ExitCode::from(2);
    }
    let total = books.len();
    let page: Vec<&Book> = books.iter().skip(cli.offset).take(cli.limit).collect();
    if cli.json {
        println!("{}", serde_json::json!({"books": page, "total": total}));
    } else {
        for b in &page {
            if cli.long { println!("{}\t{}\t{}", b.id, b.title, b.year); } else { println!("{}\t{}", b.id, b.title); }
        }
        if cli.offset + page.len() < total { println!("Next: books --offset {}", cli.offset + page.len()); }
    }
    ExitCode::SUCCESS
}
