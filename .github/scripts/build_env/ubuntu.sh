#!/bin/bash
set -euo pipefail

.github/scripts/zig.sh
.github/scripts/cargo-zigbuild.sh

PKGS=(
  libgtk-3-dev
  libayatana-appindicator3-dev
)

sudo apt update
sudo apt install -y "${PKGS[@]}"
