<h2 align="center">unrot</h2>

<div align="center">

[![CI](https://github.com/cachebag/unrot/actions/workflows/ci.yml/badge.svg)](https://github.com/cachebag/unrot/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE-MIT.md)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE-APACHE.md)

</div>

A symlink is broken when its target no longer exists. `unrot` finds these, reports the dead target path, and attempts to 
locate where it moved by fuzzy matching the target filename against the real filesystem. You decide whether to re-link, skip, or remove.

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE))
 * MIT license ([LICENSE-MIT](LICENSE-MIT))

 at your option.

