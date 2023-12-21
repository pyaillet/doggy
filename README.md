# Doggy

![Build](https://github.com/pyaillet/doggy/actions/workflows/rust.yml/badge.svg)

Minimal TUI interface for Docker

## Preview

Check a preview of the TUI:

![Preview of the TUI](./doc/preview.gif)

## How to use?

The project being in early development stage, there are no release available.
However, you can use it by building it on your own

Build it on your own:

- Install Rust (see [here](https://www.rust-lang.org/tools/install))
- Build the tool: `cargo build --release`
- Launch it: `./target/release/doggy`

## Usage

- Change view: `:` and resource name (`containers`, `images`, `networks`, `volumes`)
- Inspect container: `i` 
- Launch `/bin/bash` in the container: `s` (Error handling should be improved #22)
- Delete a resource: `Ctrl+d`
- Browse lists:
  - Up: `↑` or `j`
  - Down: `↓` or `k`
- Previous view: `Esc`

## What's working?

- [x] List containers
- [ ] Filter the container list
- [x] Inspect containers
- [x] Delete containers (running or stopped)
- [x] List images
- [ ] Filter the image list
- [x] Delete images (not used by any container)
- [x] List networks
- [x] List volumes

