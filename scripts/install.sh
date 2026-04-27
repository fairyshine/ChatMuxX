#!/usr/bin/env sh
set -eu

REPO_URL="${CHATMUXX_REPO_URL:-https://github.com/fairyshine/ChatMuxX}"
BRANCH="${CHATMUXX_BRANCH:-main}"
SRC_DIR="${CHATMUXX_SRC_DIR:-$HOME/.chatmuxx/src/ChatMuxX}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    echo "Please install $1 first, then run this installer again." >&2
    exit 1
  fi
}

need_cmd git
need_cmd cargo

mkdir -p "$(dirname "$SRC_DIR")"

if [ -d "$SRC_DIR/.git" ]; then
  echo "Updating ChatMuxX source in $SRC_DIR..."
  git -C "$SRC_DIR" fetch --prune origin
  git -C "$SRC_DIR" checkout "$BRANCH"
  git -C "$SRC_DIR" pull --ff-only origin "$BRANCH"
elif [ -e "$SRC_DIR" ]; then
  echo "Install directory exists but is not a git repo: $SRC_DIR" >&2
  echo "Move it away or set CHATMUXX_SRC_DIR to another path." >&2
  exit 1
else
  echo "Cloning ChatMuxX into $SRC_DIR..."
  git clone --branch "$BRANCH" "$REPO_URL" "$SRC_DIR"
fi

cargo install --path "$SRC_DIR/crates/cmx" --force

if command -v cmx >/dev/null 2>&1; then
  CMX_BIN="cmx"
else
  CMX_BIN="$HOME/.cargo/bin/cmx"
fi

echo
echo "ChatMuxX installed:"
"$CMX_BIN" --version

if [ ! -f "$HOME/.chatmuxx/config.toml" ]; then
  "$CMX_BIN" config init
fi

echo
echo "Next steps:"
echo "  1. Check local dependencies: $CMX_BIN doctor"
echo "  2. Login to WeChat:          $CMX_BIN login wechat"
echo "  3. Start the daemon:         $CMX_BIN daemon"
echo
echo "Update later:"
echo "  $CMX_BIN update"
echo
echo "Optional zsh setup. Copy the lines you want into ~/.zshrc:"
echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
echo "  alias cmx=\"\$HOME/.cargo/bin/cmx\""
echo
echo "Then reload zsh:"
echo "  source ~/.zshrc"
