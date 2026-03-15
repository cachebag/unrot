<h2 align="center">unrot</h2>

<div align="center">

[![CI](https://github.com/cachebag/unrot/actions/workflows/ci.yml/badge.svg)](https://github.com/cachebag/unrot/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE-MIT.md)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE-APACHE.md)

</div>

A symlink is _[broken](https://en.wikipedia.org/wiki/Symbolic_link#:~:text=Its%20target%20path%20would%20point%20to%20nothing%20and%20might%20be%20described%20as%20broken%2C%20orphaned%2C%20dead%2C%20or%20dangling)_ when its target no longer exists. `unrot` finds these, reports the dead target path, and attempts to 
locate where it moved by fuzzy matching the target filename against the real filesystem. You decide whether to re-link, skip, or remove.

## Install

```
cargo install unrot
```

## Usage

```bash
# Scans current directory for broken symlinks and interactively allows you to fix them
unrot

# Scans a specific directory
unrot -p /path/to/project

# Just list broken symlinks without the interactive resolver
unrot -p /path/to/project -l

# Search for candidates in a wider directory tree
unrot -p ~/project -s ~/

# Preview what would happen without modifying anything
unrot -p /path/to/project -d

# Add extra directories to skip (on top of .git, node_modules, target, etc.)
unrot -p /path/to/project -I vendor -I dist
```

### Interactive commands

When prompted for each broken symlink:

| Input | Action |
|-------|--------|
| `1`, `2`, ... | Re-link to the numbered candidate |
| `c` | Enter a custom path to re-link to |
| `s` | Skip this symlink |
| `r` | Remove this symlink (asks for confirmation) |

### Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--path <PATH>` | `-p` | Directory to scan for broken symlinks (default: `.`) |
| `--search-root <PATH>` | `-s` | Search for candidates here instead of the scan path |
| `--list` | `-l` | List broken symlinks and exit |
| `--dry-run` | `-d` | Preview changes without modifying the filesystem |
| `--ignore <NAME>` | `-I` | Additional directory names to skip (repeatable) |

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE))
 * MIT license ([LICENSE-MIT](LICENSE-MIT))

 at your option.

