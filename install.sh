#!/bin/sh
# upskill installer — downloads a prebuilt release binary.
#
#   curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh
#
# Windows users: run this inside WSL.
set -eu

REPO="driftsys/upskill"
BIN="upskill"
INSTALL_DIR="${UPSKILL_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${UPSKILL_VERSION:-latest}"

usage() {
    cat <<EOF
upskill installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh

Environment:
  UPSKILL_VERSION       Release tag to install (default: latest)
  UPSKILL_INSTALL_DIR   Install directory (default: \$HOME/.local/bin)

Options:
  --help, -h            Show this help and exit
EOF
}

err() {
    echo "install.sh: $*" >&2
    exit 1
}

download() {
    # $1 = url, $2 = destination path
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    else
        wget -q "$1" -O "$2"
    fi
}

verify() {
    # $1 = checksum file (sibling archive must be in the cwd)
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c "$1"
    else
        shasum -a 256 -c "$1"
    fi
}

for arg in "$@"; do
    case "$arg" in
    --help | -h)
        usage
        exit 0
        ;;
    *)
        err "unknown argument: $arg (try --help)"
        ;;
    esac
done

os="$(uname -s)"
arch="$(uname -m)"
case "${os}/${arch}" in
Linux/x86_64) target="x86_64-unknown-linux-musl" ;;
Linux/aarch64 | Linux/arm64) target="aarch64-unknown-linux-musl" ;;
Darwin/x86_64) target="x86_64-apple-darwin" ;;
Darwin/arm64) target="aarch64-apple-darwin" ;;
*) err "no prebuilt binary for ${os}/${arch}; install with: cargo install upskill" ;;
esac

if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
    err "need curl or wget to download releases"
fi
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
    err "need sha256sum or shasum to verify the download"
fi

asset="${BIN}-${target}.tar.gz"
if [ "$VERSION" = "latest" ]; then
    base="https://github.com/${REPO}/releases/latest/download"
else
    base="https://github.com/${REPO}/releases/download/${VERSION}"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

echo "Downloading ${asset} (${VERSION})..." >&2
download "${base}/${asset}" "${tmp}/${asset}" ||
    err "download failed: ${base}/${asset}"
download "${base}/${asset}.sha256" "${tmp}/${asset}.sha256" ||
    err "checksum download failed: ${base}/${asset}.sha256"

echo "Verifying checksum..." >&2
(cd "$tmp" && verify "${asset}.sha256") >/dev/null 2>&1 ||
    err "checksum verification failed for ${asset}"

tar -xzf "${tmp}/${asset}" -C "$tmp"
mkdir -p "$INSTALL_DIR"
mv "${tmp}/${BIN}" "${INSTALL_DIR}/${BIN}"
chmod +x "${INSTALL_DIR}/${BIN}"

echo "Installed ${BIN} to ${INSTALL_DIR}/${BIN}" >&2
case ":${PATH}:" in
*":${INSTALL_DIR}:"*) ;;
*)
    echo "Note: ${INSTALL_DIR} is not on your PATH. Add it with:" >&2
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" >&2
    ;;
esac

"${INSTALL_DIR}/${BIN}" --version
