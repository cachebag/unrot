use clap::Parser;
use std::path::PathBuf;
use unrot_core::{DEFAULT_IGNORE, find_broken_symlinks, find_candidates};

fn main() {
    let Args {
        list,
        path,
        search_root,
        ignore,
    } = Args::parse();

    let path = if path.as_os_str() == "." {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        path
    };

    let search_root = search_root.unwrap_or_else(|| path.clone());

    let mut all_ignore: Vec<String> = DEFAULT_IGNORE.iter().map(|s| s.to_string()).collect();
    all_ignore.extend(ignore);

    let broken = find_broken_symlinks(&path, &all_ignore);

    if list {
        for link in &broken {
            println!("{link}");
        }
        return;
    }

    for link in &broken {
        println!("{link}");
        let candidates = find_candidates(link, &search_root, &all_ignore);
        if candidates.is_empty() {
            println!("  no candidates found");
        } else {
            for (i, candidate) in candidates.iter().enumerate() {
                println!(
                    "  [{}] {} (score: {:.2})",
                    i + 1,
                    candidate.path.display(),
                    candidate.score
                );
            }
        }
        println!();
    }
}

#[derive(Parser)]
#[command(name = "unrot")]
struct Args {
    #[arg(short, long)]
    list: bool,

    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Search for candidates in this directory instead of the scan path
    #[arg(short, long)]
    search_root: Option<PathBuf>,

    /// Additional directory names to ignore during walks
    #[arg(short = 'I', long)]
    ignore: Vec<String>,
}
