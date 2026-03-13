use unrot_core::find_broken_symlinks;
use std::path::PathBuf;

fn main() {
    // FIXME: Result instead
    let path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let broken = find_broken_symlinks(&path);
    for link in broken {
        println!("{link}");
    }
}
