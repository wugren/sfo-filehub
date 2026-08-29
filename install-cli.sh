#!/bin/sh

set -eu

REPOSITORY="wugren/sfo-filehub"
API_URL="https://api.github.com/repos/${REPOSITORY}/releases/latest"
RELEASES_URL="https://github.com/${REPOSITORY}/releases/download"
DEFAULT_INSTALL_DIR="/usr/local/bin"

usage() {
    cat <<'EOF'
Usage: ./install-cli.sh [VERSION] [--install-dir DIR]

Install the latest stable filehub CLI release when VERSION is omitted, or
install a specific version such as 0.1.0 or v0.1.0.

Options:
  --install-dir DIR  Install into DIR instead of /usr/local/bin.
  -h, --help         Show this help text.
EOF
}

fail() {
    printf 'filehub installer: %s\n' "$*" >&2
    exit 1
}

require_value() {
    [ "$#" -ge 2 ] && [ -n "$2" ] || fail "$1 requires a non-empty value"
}

version_input=""
install_dir="$DEFAULT_INSTALL_DIR"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --install-dir)
            require_value "$@"
            install_dir=$2
            shift 2
            ;;
        --install-dir=*)
            install_dir=${1#*=}
            [ -n "$install_dir" ] || fail "--install-dir requires a non-empty value"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            [ "$#" -le 1 ] || fail "only one VERSION may be supplied"
            if [ "$#" -eq 1 ]; then
                [ -z "$version_input" ] || fail "only one VERSION may be supplied"
                version_input=$1
                shift
            fi
            ;;
        -*)
            fail "unknown option: $1"
            ;;
        *)
            [ -z "$version_input" ] || fail "only one VERSION may be supplied"
            version_input=$1
            shift
            ;;
    esac
done

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"
command -v install >/dev/null 2>&1 || fail "install is required"
command -v mktemp >/dev/null 2>&1 || fail "mktemp is required"

normalize_version() {
    candidate=$1
    case "$candidate" in
        v*) candidate=${candidate#v} ;;
    esac
    printf '%s\n' "$candidate" | LC_ALL=C grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
        || fail "invalid version '$1'; expected MAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH"
    printf '%s\n' "$candidate"
}

curl_https() {
    curl --fail --silent --show-error --location \
        --proto '=https' --proto-redir '=https' "$@"
}

if [ -n "$version_input" ]; then
    version=$(normalize_version "$version_input")
else
    release_json=$(curl_https \
        --header 'Accept: application/vnd.github+json' \
        "$API_URL") || fail "could not resolve the latest GitHub release; specify VERSION to retry"
    latest_tag=$(printf '%s\n' "$release_json" \
        | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        | sed -n '1p')
    [ -n "$latest_tag" ] || fail "latest GitHub release response has no tag_name; specify VERSION to retry"
    version=$(normalize_version "$latest_tag")
fi

os=$(uname -s 2>/dev/null) || fail "could not detect the operating system"
arch=$(uname -m 2>/dev/null) || fail "could not detect the CPU architecture"

case "$os:$arch" in
    Linux:x86_64|Linux:amd64)
        platform="linux-x86_64"
        ;;
    Darwin:arm64|Darwin:aarch64)
        platform="macos-aarch64"
        ;;
    Linux:*|Darwin:*)
        fail "unsupported platform $os/$arch; releases support Linux x86_64 and macOS arm64"
        ;;
    *)
        fail "unsupported operating system $os; use install-cli.ps1 on Windows"
        ;;
esac

tag="v${version}"
archive_name="filehub-cli_${version}_${platform}.tar.gz"
download_url="${RELEASES_URL}/${tag}/${archive_name}"

tmp_dir=$(mktemp -d 2>/dev/null || mktemp -d -t filehub-install) \
    || fail "could not create a temporary directory"
target_tmp=""
target_tmp_elevated=0

cleanup() {
    if [ -n "$target_tmp" ]; then
        if [ "$target_tmp_elevated" -eq 1 ] && command -v sudo >/dev/null 2>&1; then
            sudo rm -f "$target_tmp" >/dev/null 2>&1 || true
        else
            rm -f "$target_tmp" >/dev/null 2>&1 || true
        fi
    fi
    rm -rf "$tmp_dir"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

archive_path="${tmp_dir}/${archive_name}"
printf 'Downloading filehub CLI %s for %s...\n' "$version" "$platform"
curl_https --output "$archive_path" "$download_url" \
    || fail "download failed: $download_url"
[ -s "$archive_path" ] || fail "downloaded archive is empty"

archive_entries=$(tar -tzf "$archive_path") || fail "downloaded archive is not a readable tar.gz"
[ "$archive_entries" = "filehub" ] \
    || fail "archive must contain exactly one root file named filehub"
archive_listing=$(tar -tvzf "$archive_path") || fail "could not inspect the archive entry type"
case "$archive_listing" in
    -*) ;;
    *) fail "archive entry filehub must be a regular file" ;;
esac
tar -xzf "$archive_path" -C "$tmp_dir" filehub \
    || fail "could not extract filehub from the archive"
binary_path="${tmp_dir}/filehub"
[ ! -L "$binary_path" ] && [ -f "$binary_path" ] && [ -s "$binary_path" ] \
    || fail "archive did not contain a non-empty filehub binary"
chmod 0755 "$binary_path"

target_path="${install_dir%/}/filehub"

install_direct() {
    mkdir -p "$install_dir"
    target_tmp=$(mktemp "${install_dir%/}/.filehub.install.XXXXXX") \
        || fail "could not create a staging file in $install_dir"
    install -m 0755 "$binary_path" "$target_tmp" \
        || fail "could not stage filehub in $install_dir"
    mv -f "$target_tmp" "$target_path" \
        || fail "could not replace $target_path"
    target_tmp=""
}

install_elevated() {
    command -v sudo >/dev/null 2>&1 \
        || fail "$install_dir is not writable and sudo is unavailable"
    sudo mkdir -p "$install_dir" \
        || fail "could not create $install_dir with sudo"
    target_tmp=$(sudo mktemp "${install_dir%/}/.filehub.install.XXXXXX") \
        || fail "could not create a staging file in $install_dir with sudo"
    target_tmp_elevated=1
    sudo install -m 0755 "$binary_path" "$target_tmp" \
        || fail "could not stage filehub in $install_dir with sudo"
    sudo mv -f "$target_tmp" "$target_path" \
        || fail "could not replace $target_path with sudo"
    target_tmp=""
    target_tmp_elevated=0
}

if [ "$(id -u)" -eq 0 ]; then
    install_direct
elif { [ -d "$install_dir" ] && [ -w "$install_dir" ]; } \
    || { [ ! -e "$install_dir" ] && mkdir -p "$install_dir" 2>/dev/null; }; then
    install_direct
else
    install_elevated
fi

printf 'Installed filehub CLI %s to %s\n' "$version" "$target_path"
