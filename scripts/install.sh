#!/usr/bin/env sh
set -eu

REPO_URL="${CHATMUXX_REPO_URL:-https://github.com/fairyshine/ChatMuxX}"
BRANCH="${CHATMUXX_BRANCH:-main}"
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t chatmuxx)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    echo "Please install $1 first, then run this installer again." >&2
    exit 1
  fi
}

need_cmd curl
need_cmd tar
need_cmd cargo

echo "Installing ChatMuxX from $REPO_URL ($BRANCH)..."
curl -fsSL "$REPO_URL/archive/refs/heads/$BRANCH.tar.gz" -o "$TMP_DIR/chatmuxx.tar.gz"
tar -xzf "$TMP_DIR/chatmuxx.tar.gz" -C "$TMP_DIR"

SRC_DIR="$(find "$TMP_DIR" -maxdepth 1 -type d -name 'ChatMuxX-*' | head -n 1)"
if [ -z "$SRC_DIR" ]; then
  echo "Failed to unpack ChatMuxX source archive." >&2
  exit 1
fi

cargo install --path "$SRC_DIR/crates/cmux" --force

if command -v cmux >/dev/null 2>&1; then
  CMUX_BIN="cmux"
else
  CMUX_BIN="$HOME/.cargo/bin/cmux"
fi

echo
echo "ChatMuxX installed:"
"$CMUX_BIN" --version

if [ ! -f "$HOME/.chatmuxx/config.toml" ]; then
  "$CMUX_BIN" config init
fi

echo
echo "Next steps:"
echo "  1. Check local dependencies: $CMUX_BIN doctor"
echo "  2. Login to WeChat:          $CMUX_BIN login wechat"
echo "  3. Start the daemon:         $CMUX_BIN daemon"
echo
echo "If 'cmux' is not found in a new terminal, add this to your shell profile:"
echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
