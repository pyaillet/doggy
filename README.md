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

You can either install it with CRI support if you want to use it with containerd or CRI-O, or without if you only want to use it with Docker and Podman.

#### Without CRI support

- Install Rust (see [here](https://www.rust-lang.org/tools/install))
- Install the tool: `cargo install --locked --git https://github.com/pyaillet/doggy`

#### With CRI support

- Install Rust (see [here](https://www.rust-lang.org/tools/install))
- Install the dependencies
  - [protoc](https://grpc.io/docs/protoc-installation/) with the feature `cri`
- Install the tool: `cargo install --locked --git https://github.com/pyaillet/doggy --features cri`

### Use brew

- Install via homebrew

```shell-session
brew tap pyaillet/homebrew-formulas
brew install pyaillet/doggy
```

## Usage

### Docker connection

#### Linux

By default `doggy` will try the following in order:
1. Check for existence of the environment variables `DOCKER_HOST` and `DOCKER_CERT_PATH`, if both are defined it will try to connect to the address in the `DOCKER_HOST` variable and use `ca.pem`, `cert.pem` and `key.pem` in `DOCKER_CERT_PATH` to establish a secure connection to the docker daemon.
2. Check for existence of the environment variables `DOCKER_HOST`, if only this one is defined it will try to connect to the address in the `DOCKER_HOST` variable to establish *an insecure connection* to the docker daemon.
3. If the variables are not defined, it will search for the local socket `unix:///var/run/docker.sock`
4. If the socket is not found, it will search for the CRI socket `unix:///var/run/containerd/containerd.sock`

#### MacOS

By default `doggy` will check the existence of a socket file in the following in order:
1. Docker socket file `unix:///var/run/docker.sock`
2. Rancher Desktop docker socket file `unix://${HOME}/.rd/docker.sock`
3. Podman Desktop docker socket file `unix://${HOME}/.local/share/containers/podman/machine/podman.sock`
4. Orbstack docker socket file `unix://${HOME}/.orbstack/run/docker.sock`
4. Containerd CRI socket `unix:///var/run/containerd/containerd.sock`

#### Other

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

