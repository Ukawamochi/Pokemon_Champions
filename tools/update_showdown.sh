#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SHOWDOWN_DIR="$ROOT_DIR/pokemon-showdown"

if [ ! -d "$SHOWDOWN_DIR/.git" ]; then
  echo "Cloning pokemon-showdown..."
  git clone https://github.com/smogon/pokemon-showdown.git "$SHOWDOWN_DIR"
else
  echo "Updating pokemon-showdown..."
  git -C "$SHOWDOWN_DIR" pull --ff-only
fi

echo "Extracting data..."
cd "$ROOT_DIR"
node tools/extract_data.js

echo "Done."
