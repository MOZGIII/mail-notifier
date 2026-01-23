#!/bin/bash
set -euo pipefail

PKGS=(
  libgtk-3-dev
  libayatana-appindicator3-dev
)

sudo apt install -y "${PKGS[@]}"

