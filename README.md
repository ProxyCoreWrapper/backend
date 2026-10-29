# PCW backend

The PCW backend manages proxy-core configuration files and their processes. It
exposes a tonic gRPC API over a Unix domain socket.

## Features

- Create, read, update, list, and delete proxy-core configurations.
- Start, stop, and restart a configured proxy core.
- Report whether each configuration is running.
- Stream stdout and stderr from a running proxy core.
- Keep configuration metadata and process state in a local filesystem store.

The protobuf API definitions are in [`proto-files`](https://github.com/ProxyCoreWrapper/proto-files.git).

## Requirements

- Protobuf (protoc command), Rust and Cargo (edition 2024) for building from source.
- A proxy core such as `sing-box` or `xray`, if you want to start
  configurations.

## Installation

### From source

```sh
git clone https://github.com/ProxyCoreWrapper/backend.git --recurse-submodules
cd backend
cargo build --release
sudo install -Dm755 target/release/pcw-backend /usr/local/bin/pcw-backend
```

The binary can be started directly:

```sh
sudo /usr/local/bin/pcw-backend
```

### From AUR

Install the `pcw-backend` package from the [AUR](https://aur.archlinux.org/packages/pcw-backend) and start the service when it is ready with:

```sh
sudo systemctl enable --now pcw-backend.service
```

The service runs as `root` so it can start proxy cores with the privileges they
require. The package creates the `pcw` group for access to the backend socket.
Add a client user to that group when needed:

```sh
sudo usermod -aG pcw "$USER"
```

Log out and back in after changing group membership.

## Configuration

The backend looks for `/etc/pcw/backend-config.toml`. If the file does not
exist, built-in defaults are used:

```toml
[server]
uds_path = "/run/pcw/socket"

[cores]
sing-box = ["sing-box", "run", "-c", "%c"]
xray-core = ["xray", "run"]
```

`cores` maps a core tag to its command and arguments. The `%c` placeholder is
replaced with the configuration file path. If `%c` is not present, the
configuration file is passed to the process through stdin.

For example, to add another core:

```toml
[cores]
my-core = ["my-core", "run", "--config", "%c"]
```

The service stores configuration files in `/var/lib/pcw-backend/configs` and metadata in
`/var/lib/pcw-backend/meta_data.json`. These paths come from systemd's
`$STATE_DIRECTORY` environment variable.

## Running from source with a custom configuration

When running without the package, create the `pcw` group yourself or replace
it with another group. The process must run as root if its proxy cores require
root privileges.

Create the configuration directory and file before starting the binary:

```sh
sudo install -d -m755 /etc/pcw
sudo install -Dm640 -g pcw /dev/null /etc/pcw/backend-config.toml
sudoedit /etc/pcw/backend-config.toml
sudo /usr/local/bin/pcw-backend
```

## Development

Build and check the project with:

```sh
cargo check
cargo build --release
```

The generated gRPC Rust code is created by `build.rs` from the protobuf files
when Cargo builds the project.