use clap::Parser;
use std::path::PathBuf;
use unrot_core::{
    BrokenSymlink, DEFAULT_IGNORE, RepairCase, TerminalIO, find_broken_symlinks, find_candidates,
    run,
};

fn main() {
    let Args {
        list,
        path,
        search_root,
        ignore,
        dry_run,
        batch_confirm,
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

    let cases: Vec<RepairCase> = broken
        .into_iter()
        .map(|b| {
            let candidates = find_candidates(&b, &search_root, &all_ignore);
            let BrokenSymlink { link, target } = b;
            RepairCase::new(link, target, candidates)
        })
        .collect();

    if cases.is_empty() {
        println!("no broken symlinks found");
        return;
    }

    let mut io = TerminalIO;
    match run(&cases, &mut io, dry_run, batch_confirm) {
        Ok(summary) => {
            if summary.total() > 0 {
                println!("{summary}");
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
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

    /// Preview changes without modifying the filesystem
    #[arg(short, long)]
    dry_run: bool,

    /// Collect all decisions, show summary, then confirm before applying
    #[arg(short, long)]
    batch_confirm: bool,
}
