# Doggy

![Build](https://github.com/pyaillet/doggy/actions/workflows/rust.yml/badge.svg)

Minimal TUI interface for Docker

## Preview

Check a preview of the TUI:

![Preview of the TUI](./doc/preview.gif)

## How to use?

### Using the releases

You can download one of the binary from the release page
- https://github.com/pyaillet/doggy/releases/latest

### Build it on your own:

- Install Rust (see [here](https://www.rust-lang.org/tools/install))
- Build the tool: `cargo build --release`
- Launch it: `./target/release/doggy`

## Usage

- Change view: `:` and resource name (`containers`, `images`, `networks`, `volumes`)
- Show/hide stopped containers: `a`
- Inspect container: `i` 
- Launch `/bin/bash` in the container: `s` (Error handling should be improved #22)
- Launch a custom command in the container: `S`
- Delete a resource: `Ctrl+d`
- Browse lists:
  - Up: `↑` or `j`
  - Down: `↓` or `k`
- Previous view: `Esc`

## What's working? (on the main branch)

- [x] List containers
- [x] Display the stopped containers
- [ ] Filter the container list
- [x] Inspect containers
- [x] Exec `/bin/bash` in a container
- [x] Delete containers (running or stopped)
- [x] List images
- [ ] Filter the image list
- [x] Delete images (not used by any container)
- [x] List networks
- [x] Delete network
- [x] List volumes
- [x] Delete volume

