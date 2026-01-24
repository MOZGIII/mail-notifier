# macOS support utilities

## Prerequisites

```shell
cargo install --git "https://github.com/burtonageo/cargo-bundle" --rev 3a1cf9d9
```

Install the specific version of `cargo-bundle` from git;
`0.9.0` (latest released version at this time) doesn't have the the changes we
rely on here (<https://github.com/burtonageo/cargo-bundle/pull/147>).

## Installation

```shell
cargo bundle -p tray --release
sudo cp -r target/release/bundle/osx/MailNotifierTray.app /Applications
./macos/tray/svc-install
```

## Uninstallation

```shell
./macos/tray/svc-uninstall
sudo rm -rf /Applications/MailNotifierTray.app
```
