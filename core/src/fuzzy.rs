use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::scanner::BrokenSymlink;

pub fn find_candidates(broken: &BrokenSymlink, search_root: &Path) -> Vec<PathBuf> {
    let target_name = match broken.target.file_name() {
        Some(name) => name.to_os_string(),
        None => return vec![],
    };

    let mut exact = Vec::new();
    let mut partial = Vec::new();

    for entry in WalkDir::new(search_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path() == broken.link {
            continue;
        }

        let name = entry.file_name();

        if name == target_name {
            exact.push(entry.into_path());
        } else if let (Some(n), Some(t)) = (name.to_str(), target_name.to_str())
            && (n.contains(t) || t.contains(n))
        {
            partial.push(entry.into_path());
        }
    }

    exact.extend(partial);
    exact
}
