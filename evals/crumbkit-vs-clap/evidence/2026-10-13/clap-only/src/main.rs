use std::process::ExitCode;
use clap::Parser;
use serde::Serialize;

#[derive(Serialize, Clone)]
struct Book { id: u32, title: &'static str, author: &'static str, year: u16 }

#[derive(Parser)]
#[command(name = "books")]
struct Cli {
    #[arg(long)] author: Option<String>,
    #[arg(long, global = true)] json: bool,
    #[arg(long, global = true)] long: bool,
    #[arg(long, default_value_t = 2, conflicts_with = "all")] limit: usize,
    #[arg(long)] all: bool,
}
#[derive(Serialize)]
struct Page<T> { schema: &'static str, data: Vec<T>, total: usize, truncated: bool }

fn main() -> ExitCode {
    let cli = Cli::parse();
    let books = vec![
        Book { id: 1, title: "Pride and Prejudice", author: "Jane Austen", year: 1813 },
        Book { id: 2, title: "Frankenstein", author: "Mary Shelley", year: 1818 },
        Book { id: 3, title: "Moby-Dick", author: "Herman Melville", year: 1851 },
    ];
    let books: Vec<Book> = books.into_iter().filter(|b| cli.author.as_deref().is_none_or(|a| b.author == a)).collect();
    if let (Some(author), true) = (&cli.author, books.is_empty()) {
        if cli.json {
            println!("{}", serde_json::json!({"schema":"crumbkit/failure/v1","code":"shelf::books::unknown_author","message":format!("the catalogue has no books by {author}"),"recovery":{"kind":"run","call":"books"}}));
        } else {
            eprintln!("error: the catalogue has no books by {author}\n  help:  run `books`\n  code:  shelf::books::unknown_author");
        }
        return ExitCode::from(1);
    }
    let total = books.len();
    let limit = if cli.all { total } else { cli.limit };
    let page: Vec<Book> = books.into_iter().take(limit).collect();
    let truncated = page.len() < total;
    if cli.json {
        let doc = Page { schema: "shelf/books-list/v1", data: page, total, truncated };
        println!("{}", serde_json::to_string(&doc).unwrap());
    } else {
        // Column widths
        let mut rows: Vec<Vec<String>> = vec![if cli.long { vec!["ID".into(),"TITLE".into(),"YEAR".into()] } else { vec!["ID".into(),"TITLE".into()] }];
        for b in &page { let mut r = vec![b.id.to_string(), b.title.to_string()]; if cli.long { r.push(b.year.to_string()); } rows.push(r); }
        let n = rows[0].len();
        let widths: Vec<usize> = (0..n).map(|i| rows.iter().map(|r| r[i].len()).max().unwrap_or(0)).collect();
        for r in &rows {
            let line: Vec<String> = r.iter().enumerate().map(|(i, c)| if i + 1 == n { c.clone() } else { format!("{:<w$}", c, w = widths[i]) }).collect();
            println!("{}", line.join("  ").trim_end());
        }
        if truncated { eprintln!("Showing {} of {} books. --all shows every one.", page.len(), total); }
    }
    ExitCode::SUCCESS
}
