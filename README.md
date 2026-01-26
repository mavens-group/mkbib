<div align="center">

  <img src="assets/mkbib.svg" alt="MkBib Logo" width="120" height="120">

  # MkBib

  **A high-performance BibTeX manager for the Rust ecosystem.**

  [![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
  [![Rust](https://img.shields.io/badge/Built%20With-Rust-grey)](https://www.rust-lang.org/)
  [![CI](https://github.com/mavensgroup/mkbib-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/mavensgroup/mkbib-rs/actions)

</div>

---

## Overview

**MkBib** is a bibliography manager engineered for performance and correctness. Built with **Rust** and **GTK4**, it provides a strictly typed environment for managing `.bib` databases, ensuring data integrity before compilation.

Designed for researchers and technical writers, MkBib focuses on handling large reference datasets with zero latency, offering real-time filtering and BibLaTeX compliance validation.

## Key Features

* **Performance:** Native code backend ensures instant load times for large databases (10,000+ entries).
* **Strict Validation:** Enforces BibLaTeX field requirements to prevent compilation errors.
* **Atomic Saves:** Guarantees no data corruption during write operations.
* **Modern Interface:** A distraction-free, responsive GTK4 environment that respects system themes.

## Installation

### Dependencies
MkBib requires **GTK4** and **LibAdwaita**. Ensure development headers are installed before building.

**Ubuntu / Debian**
```bash
sudo apt update
sudo apt install build-essential libgtk-4-dev libadwaita-1-dev

```

**Fedora / RHEL**

```bash
sudo dnf install gtk4-devel libadwaita-devel gcc

```

**Arch Linux**

```bash
sudo pacman -S gtk4 libadwaita base-devel

```

### Building from Source

```bash
git clone [https://github.com/mavensgroup/mkbib-rs.git](https://github.com/mavensgroup/mkbib-rs.git)
cd mkbib-rs
cargo install --path .

```

## Usage

**MkBib** can be launched via the terminal or your application menu.

```bash
mkbib-rs [OPTIONS] [FILE]

```

* **Open Library:** `File > Open` or `Ctrl+O`
* **Search:** Filter entries by `Author`, `Year`, or `Title` using the search bar.
* **Edit:** Double-click any entry to modify fields.

## Citation

If you use MkBib in your research workflow, please cite the software:

```bibtex
@software{mkbib2025,
  author = {Mavens Group},
  title = {MkBib: High-Performance BibTeX Manager},
  year = {2025},
  url = {[https://github.com/mavensgroup/mkbib-rs](https://github.com/mavensgroup/mkbib-rs)},
  version = {0.1.0}
}

```

## Contributing

Contributions are welcome. Please refer to `CONTRIBUTING.md` for guidelines on submitting patches and reporting issues.

## License

This project is licensed under the **GPL-3.0 License**. See the `LICENSE` file for details.
