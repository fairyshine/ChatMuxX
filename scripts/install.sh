#!/usr/bin/env sh
set -eu

REPO_URL="${CHATMUXX_REPO_URL:-https://github.com/fairyshine/ChatMuxX}"
REPO_API_URL="${CHATMUXX_REPO_API_URL:-https://api.github.com/repos/fairyshine/ChatMuxX}"
RELEASES_PAGE_URL="${CHATMUXX_RELEASES_PAGE_URL:-https://github.com/fairyshine/ChatMuxX/releases}"
RELEASE_BASE_URL="${CHATMUXX_RELEASE_BASE_URL:-https://github.com/fairyshine/ChatMuxX/releases/download}"
BRANCH="${CHATMUXX_BRANCH:-master}"
SRC_DIR="${CHATMUXX_SRC_DIR:-$HOME/.chatmuxx/src/ChatMuxX}"
INSTALL_DIR="${CHATMUXX_INSTALL_DIR:-$HOME/.cargo/bin}"
INSTALL_METHOD="${CHATMUXX_INSTALL_METHOD:-release}"
VERSION="${CHATMUXX_VERSION:-latest-prerelease}"
GITHUB_USER_AGENT="${CHATMUXX_GITHUB_USER_AGENT:-ChatMuxX installer}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    return 1
  fi
}

github_get() {
  curl -fsSL -H "User-Agent: $GITHUB_USER_AGENT" "$@"
}

github_effective_url() {
  curl -fsSL -H "User-Agent: $GITHUB_USER_AGENT" -o /dev/null -w '%{url_effective}' "$@"
}

detect_asset() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m | tr '[:upper:]' '[:lower:]')"
  bin_name="cmx"

  case "$os" in
    darwin)
      case "$arch" in
        x86_64 | amd64) asset="macos-x86_64" ;;
        arm64 | aarch64) asset="macos-aarch64" ;;
        *) return 1 ;;
      esac
      ;;
    linux)
      case "$arch" in
        x86_64 | amd64) asset="linux-x86_64" ;;
        arm64 | aarch64) asset="linux-aarch64" ;;
        *) return 1 ;;
      esac
      ;;
    mingw* | msys* | cygwin*)
      bin_name="cmx.exe"
      case "$arch" in
        x86_64 | amd64) asset="windows-x86_64" ;;
        arm64 | aarch64) asset="windows-aarch64" ;;
        *) return 1 ;;
      esac
      ;;
    *)
      return 1
      ;;
  esac

  printf '%s %s\n' "$asset" "$bin_name"
}

latest_prerelease_tag() {
  need_cmd curl || return 1
  need_cmd sed || return 1
  need_cmd head || return 1
  need_cmd tr || return 1

  if page="$(github_get "$RELEASES_PAGE_URL" 2>/dev/null)"; then
    tag="$(printf '%s\n' "$page" \
      | sed -n 's#.*href="/fairyshine/ChatMuxX/releases/tag/\([^"?/]*\)".*#\1#p' \
      | head -n 1 || true)"
    if [ -n "$tag" ]; then
      printf '%s\n' "$tag"
      return 0
    fi
  fi

  if json="$(github_get "$REPO_API_URL/releases" 2>/dev/null)"; then
    printf '%s\n' "$json" \
      | tr ',' '\n' \
      | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
      | head -n 1
  fi
}

latest_stable_tag() {
  need_cmd curl || return 1
  need_cmd sed || return 1

  if effective_url="$(github_effective_url "$RELEASES_PAGE_URL/latest" 2>/dev/null)"; then
    case "$effective_url" in
      */releases/tag/*)
        tag="${effective_url##*/releases/tag/}"
        tag="${tag%%\?*}"
        if [ -n "$tag" ] && [ "$tag" != "latest" ]; then
          printf '%s\n' "$tag"
          return 0
        fi
        ;;
    esac
  fi

  if json="$(github_get "$REPO_API_URL/releases/latest" 2>/dev/null)"; then
    printf '%s\n' "$json" \
      | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
      | head -n 1
  fi
}

sha256_of() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    return 1
  fi
}

verify_checksum() {
  checksum_archive_path="$1"
  checksum_file_path="$2"

  if ! command -v awk >/dev/null 2>&1; then
    echo "Skipping checksum verification because awk is missing."
    return 0
  fi

  expected="$(awk '{print $1; exit}' "$checksum_file_path")"
  if [ -z "$expected" ]; then
    echo "Downloaded checksum file is empty." >&2
    return 1
  fi

  if actual="$(sha256_of "$checksum_archive_path")"; then
    if [ "$actual" = "$expected" ]; then
      return 0
    fi
    echo "Checksum mismatch for $(basename "$checksum_archive_path")." >&2
    return 1
  fi

  echo "Skipping checksum verification because shasum/sha256sum is missing."
  return 0
}

install_release() {
  need_cmd curl || return 1
  need_cmd tar || return 1
  need_cmd uname || return 1
  need_cmd mktemp || return 1

  detected="$(detect_asset)" || {
    echo "Unsupported platform: $(uname -s) $(uname -m)" >&2
    return 1
  }
  set -- $detected
  asset="$1"
  bin_name="$2"

  case "$VERSION" in
    latest | latest-prerelease)
      tag="$(latest_prerelease_tag)"
      if [ -z "$tag" ]; then
        echo "Could not find a GitHub release for ChatMuxX." >&2
        return 1
      fi
      ;;
    latest-stable)
      tag="$(latest_stable_tag)"
      if [ -z "$tag" ]; then
        echo "Could not find a stable GitHub release for ChatMuxX." >&2
        return 1
      fi
      ;;
    v*) tag="$VERSION" ;;
    *) tag="v$VERSION" ;;
  esac

  version="${tag#v}"
  archive="cmx-v${version}-${asset}.tar.gz"
  url="${RELEASE_BASE_URL}/${tag}/${archive}"
  checksum_url="${url}.sha256"

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT INT HUP TERM

  echo "Installing ChatMuxX ${tag} from GitHub Release..."
  echo "Downloading ${archive}..."
  github_get "$url" -o "$tmp_dir/$archive" || return 1
  github_get "$checksum_url" -o "$tmp_dir/$archive.sha256" || return 1
  verify_checksum "$tmp_dir/$archive" "$tmp_dir/$archive.sha256" || return 1

  mkdir -p "$tmp_dir/package"
  tar -xzf "$tmp_dir/$archive" -C "$tmp_dir/package" || return 1

  if [ ! -f "$tmp_dir/package/$bin_name" ]; then
    echo "Release archive does not contain $bin_name." >&2
    return 1
  fi

  mkdir -p "$INSTALL_DIR"
  cp "$tmp_dir/package/$bin_name" "$INSTALL_DIR/$bin_name"
  chmod +x "$INSTALL_DIR/$bin_name"
}

install_source() {
  need_cmd git || return 1
  need_cmd cargo || return 1

  mkdir -p "$(dirname "$SRC_DIR")"

  if [ -d "$SRC_DIR/.git" ]; then
    echo "Updating ChatMuxX source in $SRC_DIR..."
    git -C "$SRC_DIR" fetch --prune origin
    git -C "$SRC_DIR" checkout "$BRANCH"
    git -C "$SRC_DIR" pull --ff-only origin "$BRANCH"
  elif [ -e "$SRC_DIR" ]; then
    echo "Install directory exists but is not a git repo: $SRC_DIR" >&2
    echo "Move it away or set CHATMUXX_SRC_DIR to another path." >&2
    return 1
  else
    echo "Cloning ChatMuxX into $SRC_DIR..."
    git clone --branch "$BRANCH" "$REPO_URL" "$SRC_DIR"
  fi

  cargo install --path "$SRC_DIR/crates/cmx" --force
}

source_install_hint() {
  echo
  echo "Release install failed."
  echo
  echo "You can install from source instead. Source install requires git and Rust/Cargo:"
  echo
  echo "  curl -fsSL https://raw.githubusercontent.com/fairyshine/ChatMuxX/master/scripts/install.sh -o /tmp/chatmuxx-install.sh"
  echo "  CHATMUXX_INSTALL_METHOD=source sh /tmp/chatmuxx-install.sh"
  echo
  echo "You can also select a specific release version:"
  echo
  echo "  CHATMUXX_VERSION=v0.0.1-dev1 sh /tmp/chatmuxx-install.sh"
  echo
  echo "Pre-releases are supported. The default is CHATMUXX_VERSION=latest-prerelease."
}

find_cmx() {
  if command -v cmx >/dev/null 2>&1; then
    command -v cmx
  elif command -v cmx.exe >/dev/null 2>&1; then
    command -v cmx.exe
  elif [ -x "$INSTALL_DIR/cmx" ]; then
    printf '%s\n' "$INSTALL_DIR/cmx"
  elif [ -x "$INSTALL_DIR/cmx.exe" ]; then
    printf '%s\n' "$INSTALL_DIR/cmx.exe"
  else
    printf '%s\n' "$INSTALL_DIR/cmx"
  fi
}

case "$INSTALL_METHOD" in
  release)
    if ! install_release; then
      source_install_hint
      exit 1
    fi
    ;;
  source)
    if ! install_source; then
      echo
      echo "Source install failed." >&2
      exit 1
    fi
    ;;
  *)
    echo "Unknown CHATMUXX_INSTALL_METHOD: $INSTALL_METHOD" >&2
    echo "Use 'release' or 'source'." >&2
    exit 1
    ;;
esac

CMX_BIN="$(find_cmx)"

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
echo "  rerun this installer to download the latest release, including pre-releases"
echo
echo "Optional zsh setup. Copy the lines you want into ~/.zshrc:"
echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
echo "  alias cmx=\"\$HOME/.cargo/bin/cmx\""
echo
echo "Then reload zsh:"
echo "  source ~/.zshrc"
