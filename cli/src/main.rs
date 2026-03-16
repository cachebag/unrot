use clap::{Parser, Subcommand};
use std::path::PathBuf;
use unrot_core::{
    BrokenSymlink, DEFAULT_IGNORE, RepairCase, TerminalIO, find_broken_symlinks, find_candidates,
    run,
};

fn main() {
    let cli = Cli::parse();

    let (mode, path, fix_opts) = match &cli.subcommand {
        Some(Sub::Scan(s)) => (Mode::Scan, resolve_path(&s.path), None),
        Some(Sub::Fix(f)) => (
            Mode::Fix,
            resolve_path(&f.path),
            Some(FixOptions {
                search_root: f.search_root.clone(),
                ignore: f.ignore.clone(),
                dry_run: f.dry_run,
                batch_confirm: f.batch_confirm,
            }),
        ),
        Some(Sub::List(l)) => (Mode::List, resolve_path(&l.path), None),
        None => (
            Mode::Fix,
            resolve_path(&cli.path),
            Some(FixOptions {
                search_root: cli.search_root.clone(),
                ignore: cli.ignore.clone(),
                dry_run: cli.dry_run,
                batch_confirm: cli.batch_confirm,
            }),
        ),
    };

    let search_root = fix_opts
        .as_ref()
        .and_then(|o| o.search_root.clone())
        .unwrap_or_else(|| path.clone());

    let mut all_ignore: Vec<String> = DEFAULT_IGNORE.iter().map(|s| s.to_string()).collect();
    if let Some(opts) = &fix_opts {
        all_ignore.extend(opts.ignore.iter().cloned());
    }

    let broken = find_broken_symlinks(&path, &all_ignore);

    match mode {
        Mode::List => {
            for link in &broken {
                println!("{}", link.link.display());
            }
            return;
        }
        Mode::Scan => {
            for b in &broken {
                println!("{b}");
            }
            return;
        }
        Mode::Fix => {}
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

    let dry_run = fix_opts.as_ref().map(|o| o.dry_run).unwrap_or(false);
    let batch_confirm = fix_opts.as_ref().map(|o| o.batch_confirm).unwrap_or(false);

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

fn resolve_path(path: &std::path::Path) -> PathBuf {
    if path.as_os_str() == "." {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    } else {
        path.to_path_buf()
    }
}

enum Mode {
    Scan,
    Fix,
    List,
}

struct FixOptions {
    search_root: Option<PathBuf>,
    ignore: Vec<String>,
    dry_run: bool,
    batch_confirm: bool,
}

#[derive(Parser)]
#[command(name = "unrot")]
struct Cli {
    /// Path to scan for broken symlinks (used when no subcommand)
    #[arg(default_value = ".")]
    path: PathBuf,

    #[command(subcommand)]
    subcommand: Option<Sub>,

    /// Search for candidates in this directory instead of the scan path
    #[arg(short, long)]
    search_root: Option<PathBuf>,

    /// Additional directory names to ignore during walks
    #[arg(short = 'I', long)]
    ignore: Vec<String>,

    /// Preview changes without modifying the filesystem
    #[arg(long)]
    dry_run: bool,

    /// Collect all decisions, show summary, then confirm before applying
    #[arg(long)]
    batch_confirm: bool,
}

#[derive(Subcommand)]
enum Sub {
    /// Scan only, report broken links (link -> target)
    Scan(ScanArgs),

    /// Interactive fix mode (default)
    Fix(FixArgs),

    /// List broken link paths only, no candidates, exit
    List(ListArgs),
}

#[derive(clap::Args)]
struct ScanArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(clap::Args)]
struct FixArgs {
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Search for candidates in this directory instead of the scan path
    #[arg(short, long)]
    search_root: Option<PathBuf>,

    /// Additional directory names to ignore during walks
    #[arg(short = 'I', long)]
    ignore: Vec<String>,

    /// Preview changes without modifying the filesystem
    #[arg(long)]
    dry_run: bool,

    /// Collect all decisions, show summary, then confirm before applying
    #[arg(long)]
    batch_confirm: bool,
}

#[derive(clap::Args)]
struct ListArgs {
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        let mut a = vec!["unrot"];
        a.extend(args);
        Cli::try_parse_from(a)
    }

    #[test]
    fn unrot_no_args_defaults_to_fix_cwd() {
        let cli = parse(&[]).unwrap();
        assert!(cli.subcommand.is_none());
        assert_eq!(cli.path, PathBuf::from("."));
    }

    #[test]
    fn unrot_path_fix_mode() {
        let cli = parse(&["/tmp"]).unwrap();
        assert!(cli.subcommand.is_none());
        assert_eq!(cli.path, PathBuf::from("/tmp"));
    }

    #[test]
    fn unrot_scan_default_path() {
        let cli = parse(&["scan"]).unwrap();
        match &cli.subcommand {
            Some(Sub::Scan(s)) => assert_eq!(s.path, PathBuf::from(".")),
            _ => panic!("expected Scan subcommand"),
        }
    }

    #[test]
    fn unrot_scan_with_path() {
        let cli = parse(&["scan", "/tmp"]).unwrap();
        match &cli.subcommand {
            Some(Sub::Scan(s)) => assert_eq!(s.path, PathBuf::from("/tmp")),
            _ => panic!("expected Scan subcommand"),
        }
    }

    #[test]
    fn unrot_fix_default_path() {
        let cli = parse(&["fix"]).unwrap();
        match &cli.subcommand {
            Some(Sub::Fix(f)) => assert_eq!(f.path, PathBuf::from(".")),
            _ => panic!("expected Fix subcommand"),
        }
    }

    #[test]
    fn unrot_fix_with_path() {
        let cli = parse(&["fix", "/tmp"]).unwrap();
        match &cli.subcommand {
            Some(Sub::Fix(f)) => assert_eq!(f.path, PathBuf::from("/tmp")),
            _ => panic!("expected Fix subcommand"),
        }
    }

    #[test]
    fn unrot_list_default_path() {
        let cli = parse(&["list"]).unwrap();
        match &cli.subcommand {
            Some(Sub::List(l)) => assert_eq!(l.path, PathBuf::from(".")),
            _ => panic!("expected List subcommand"),
        }
    }

    #[test]
    fn unrot_list_with_path() {
        let cli = parse(&["list", "/var"]).unwrap();
        match &cli.subcommand {
            Some(Sub::List(l)) => assert_eq!(l.path, PathBuf::from("/var")),
            _ => panic!("expected List subcommand"),
        }
    }

    #[test]
    fn unrot_fix_dry_run() {
        let cli = parse(&["fix", "/tmp", "--dry-run"]).unwrap();
        match &cli.subcommand {
            Some(Sub::Fix(f)) => assert!(f.dry_run),
            _ => panic!("expected Fix subcommand"),
        }
    }

    #[test]
    fn unrot_fix_search_root() {
        let cli = parse(&["fix", "/tmp", "-s", "/other"]).unwrap();
        match &cli.subcommand {
            Some(Sub::Fix(f)) => assert_eq!(
                f.search_root.as_deref(),
                Some(std::path::Path::new("/other"))
            ),
            _ => panic!("expected Fix subcommand"),
        }
    }

    #[test]
    fn unrot_default_ignore() {
        let cli = parse(&["/tmp", "-I", ".git"]).unwrap();
        assert!(cli.subcommand.is_none());
        assert_eq!(cli.ignore, vec![".git"]);
    }
}
