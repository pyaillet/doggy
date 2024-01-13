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
- Install the dependencies
  - [protoc](https://grpc.io/docs/protoc-installation/) with the feature `cri` which as activated by default
- Build the tool: `cargo build --release`
- Launch it: `./target/release/doggy`

### Use brew

- Install via homebrew

```shell-session
brew tap pyaillet/homebrew-formulas
brew install pyaillet/doggy
```

## Usage

### Docker connection

By default `doggy` will try the following in order:
1. Check for existence of the environment variables `DOCKER_HOST` and `DOCKER_CERT_PATH`, if both are defined it will try to connect to the address in the `DOCKER_HOST` variable and use `ca.pem`, `cert.pem` and `key.pem` in `DOCKER_CERT_PATH` to establish a secure connection to the docker daemon.
2. Check for existence of the environment variables `DOCKER_HOST`, if only this one is defined it will try to connect to the address in the `DOCKER_HOST` variable to establish *an insecure connection* to the docker daemon.
3. If the variables are not defined, it will search for the local socket `unix:///var/run/docker.sock`
4. If the socket is not found, it will search for the CRI socket `unix:///var/run/containerd/containerd.sock`

It's also possible to specify where to find the sockets with command args:
- `--docker <docker socket path>`
- `--cri <cri socket path>`

### Key bindings

- Display help screen: `?`
- Change view: `:` and resource name (`containers`, `images`, `networks`, `volumes`)
- Filter resources by name: `/`
- Container view:
  - Show/hide stopped containers: `a`
  - Launch `/bin/bash` in the container: `s`
  - Launch a custom command in the container: `S`
  - Show container logs: `l`
- Sort by columns: `F[1234]`
- Inspect resource: `i` 
- Delete a resource: `Ctrl+d`
- Browse lists:
  - Up: `↑` or `j`
  - Down: `↓` or `k`
- Previous view: `Esc`

## What's working? (on the main branch)

- [x] List containers
- [x] Display the stopped containers
- [x] Filter the container list
- [x] Inspect containers
- [x] View container logs
- [x] Exec `/bin/bash` in a container
- [x] Delete containers (running or stopped)
- [x] List images
- [x] Inspect image
- [x] Filter the image list
- [x] Delete images (not used by any container)
- [x] List networks
- [x] Inspect network
- [x] Filter the network list
- [x] Delete network
- [x] List volumes
- [x] Inspect volume
- [x] Filter the volume list
- [x] Delete volume

