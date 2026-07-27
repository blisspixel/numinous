#!/bin/sh
# Numinous installer for macOS and Linux. One line to play:
#
#   curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
#
# What it does, in order: downloads the latest published release for this
# machine, verifies both archive checksums and closed payload manifests, puts
# numinous, numinous-app, and numinous-mcp in ~/.numinous/bin, installs the
# built-in radio once, and adds that directory to PATH.
#
# Re-run it any time to update. Remove everything it installed with:
#
#   curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh -s -- --uninstall
#
# Uninstalling never touches play history: ~/.numinous-journey,
# ~/.numinous-scores, and ~/.numinous-cairn stay yours.
#
# Options: --uninstall, --no-modify-path, --adopt-legacy, --source,
# --self-test, --help.
# Set NUMINOUS_HOME to install somewhere other than ~/.numinous.
set -eu

REPO="blisspixel/numinous"
REPO_URL="https://github.com/${REPO}"
REPO_API_URL="https://api.github.com/repos/${REPO}"
SNAPSHOT_URL="https://codeload.github.com/${REPO}/tar.gz/refs/heads/main"
INSTALL_SH_URL="https://raw.githubusercontent.com/${REPO}/main/scripts/install.sh"
INSTALL_PS1_URL="https://raw.githubusercontent.com/${REPO}/main/scripts/install.ps1"
NUMINOUS_HOME="${NUMINOUS_HOME:-$HOME/.numinous}"
INSTALL_MARKER_TEXT='Numinous install root v2'
LEGACY_INSTALL_MARKER_TEXT='Numinous install root'
INSTALLER_NOTE='added by the Numinous installer'

say() { printf '%s\n' "$1"; }
fail() {
    printf 'numinous install: %s\n' "$1" >&2
    exit 1
}
have() { command -v "$1" >/dev/null 2>&1; }
INVOCATION_DIR="$(pwd -P)" || fail "could not resolve the starting directory"

posix_quote() {
    printf "'"
    printf '%s' "$1" | sed "s/'/'\\\\''/g"
    printf "'"
}

fish_quote() {
    printf "'"
    printf '%s' "$1" | sed "s/\\\\/\\\\\\\\/g; s/'/\\\\'/g"
    printf "'"
}

directory_is_empty() {
    for entry in "$1"/.[!.]* "$1"/..?* "$1"/*; do
        if [ -e "$entry" ] || [ -L "$entry" ]; then
            return 1
        fi
    done
    return 0
}

legacy_install_marker_is_valid() {
    [ -f "$1/.numinous-install-root" ] || return 1
    [ ! -L "$1/.numinous-install-root" ] || return 1
    marker_size="$(wc -c <"$1/.numinous-install-root" | tr -d '[:space:]')"
    [ "$marker_size" = 22 ] || return 1
    [ "$(cat "$1/.numinous-install-root")" = "$LEGACY_INSTALL_MARKER_TEXT" ]
}

stat_owner_mode_identity() {
    case "$(uname -s)" in
        Darwin) stat -f '%u %Lp %d:%i' "$1" ;;
        *) stat -c '%u %a %d:%i' "$1" ;;
    esac
}

self_test_without_posix_modes() {
    [ "${SELF_TEST:-0}" -eq 1 ] || return 1
    case "$(uname -s)" in
        MINGW* | MSYS* | CYGWIN*) return 0 ;;
        *) return 1 ;;
    esac
}

mode_is() {
    [ "$1" = "$2" ] || self_test_without_posix_modes
}

install_receipt_dir() {
    printf '%s' "$HOME/.config/numinous/install-roots"
}

install_marker_token() {
    marker="$1/.numinous-install-root"
    [ -f "$marker" ] && [ ! -L "$marker" ] || return 1
    [ "$(wc -l <"$marker" | tr -d '[:space:]')" = 2 ] || return 1
    [ "$(sed -n '1p' "$marker")" = "$INSTALL_MARKER_TEXT" ] || return 1
    token="$(sed -n '2p' "$marker")"
    case "$token" in
        root.[A-Za-z0-9][A-Za-z0-9][A-Za-z0-9][A-Za-z0-9][A-Za-z0-9][A-Za-z0-9])
            printf '%s' "$token"
            ;;
        *) return 1 ;;
    esac
}

install_root_identity() {
    root="$1"
    root_physical="$(CDPATH= cd -P "$root" 2>/dev/null && pwd)" || return 1
    set -- $(stat_owner_mode_identity "$root_physical")
    [ "$#" = 3 ] || return 1
    owner="$1"
    mode="$2"
    identity="$3"
    if self_test_without_posix_modes; then
        mode=700
    fi
    printf '%s\n%s\n%s\n' "$owner" "$mode" "$identity" "$root_physical"
}

install_marker_is_valid() {
    root="$1"
    token="$(install_marker_token "$root")" || return 1
    set -- $(stat_owner_mode_identity "$root/.numinous-install-root")
    [ "$#" = 3 ] && [ "$1" = "$(id -u)" ] && mode_is "$2" 600 || return 1
    receipt_dir="$(install_receipt_dir)"
    receipt="$receipt_dir/$token"
    [ -d "$receipt_dir" ] && [ ! -L "$receipt_dir" ] \
        && [ -f "$receipt" ] && [ ! -L "$receipt" ] || return 1
    set -- $(stat_owner_mode_identity "$receipt_dir")
    [ "$#" = 3 ] && [ "$1" = "$(id -u)" ] && mode_is "$2" 700 || return 1
    set -- $(stat_owner_mode_identity "$receipt")
    [ "$#" = 3 ] && [ "$1" = "$(id -u)" ] && mode_is "$2" 600 || return 1
    expected="$(install_root_identity "$root")" || return 1
    [ "$(cat "$receipt")" = "$expected" ]
}

claim_install_root() (
    root="$1"
    [ -d "$root" ] && [ ! -L "$root" ] || fail "cannot claim a non-directory install root"
    marker_path="$root/.numinous-install-root"
    if [ -e "$marker_path" ] || [ -L "$marker_path" ]; then
        [ -f "$marker_path" ] && [ ! -L "$marker_path" ] \
            || fail "the install marker destination is not a regular file"
    fi
    owner="$(stat_owner_mode_identity "$root")" || fail "cannot inspect the install root"
    set -- $owner
    [ "$#" = 3 ] && [ "$1" = "$(id -u)" ] \
        || fail "NUMINOUS_HOME must be owned by the current user"
    chmod 700 "$root" 2>/dev/null \
        || self_test_without_posix_modes \
        || fail "could not make NUMINOUS_HOME private"

    receipt_dir="$(install_receipt_dir)"
    old_umask="$(umask)"
    receipt=''
    marker_stage=''
    published_marker=0
    trap '
        [ -z "$receipt" ] || rm -f -- "$receipt" || true
        [ -z "$marker_stage" ] || rm -f -- "$marker_stage" || true
        [ "$published_marker" -eq 0 ] \
            || rm -f -- "$marker_path" \
            || true
        umask "$old_umask"
    ' EXIT HUP INT TERM
    umask 077
    mkdir -p "$receipt_dir" || fail "could not create the private install receipt directory"
    [ ! -L "$receipt_dir" ] || fail "the install receipt directory must not be a symbolic link"
    chmod 700 "$receipt_dir" 2>/dev/null \
        || self_test_without_posix_modes \
        || fail "could not protect the install receipt directory"
    receipt="$(mktemp "$receipt_dir/root.XXXXXX")" \
        || fail "could not create a private install receipt"
    token="${receipt##*/}"
    install_root_identity "$root" >"$receipt" \
        || fail "could not record the physical install root identity"
    chmod 600 "$receipt" 2>/dev/null \
        || self_test_without_posix_modes \
        || fail "could not protect the install receipt"

    marker_stage="$(mktemp "$root/.numinous-marker.XXXXXX")" \
        || fail "could not create the install marker"
    printf '%s\n%s\n' "$INSTALL_MARKER_TEXT" "$token" >"$marker_stage"
    chmod 600 "$marker_stage" 2>/dev/null \
        || self_test_without_posix_modes \
        || fail "could not protect the install marker"
    mv -f -- "$marker_stage" "$marker_path"
    marker_stage=''
    published_marker=1
    install_marker_is_valid "$root" || fail "the install-root identity could not be verified"
    receipt=''
    published_marker=0
    umask "$old_umask"
    trap - EXIT HUP INT TERM
)

legacy_install_is_valid() (
    root="$1"
    [ -d "$root/src" ] && [ ! -L "$root/src" ] \
        && [ -d "$root/bin" ] && [ ! -L "$root/bin" ] \
        && [ -f "$root/src/Cargo.toml" ] && [ ! -L "$root/src/Cargo.toml" ] \
        || return 1
    for binary in numinous numinous-app numinous-mcp; do
        [ -f "$root/bin/$binary" ] && [ ! -L "$root/bin/$binary" ] || return 1
    done
    for entry in "$root"/.[!.]* "$root"/..?* "$root"/*; do
        if [ ! -e "$entry" ] && [ ! -L "$entry" ]; then
            continue
        fi
        case "$entry" in
            "$root/src" | "$root/bin") ;;
            "$root/.numinous-install-root")
                legacy_install_marker_is_valid "$root" || return 1
                ;;
            *) return 1 ;;
        esac
    done
)

validate_install_root() {
    case "$NUMINOUS_HOME" in
        "" | / | "$HOME") fail "NUMINOUS_HOME must name a dedicated absolute directory" ;;
        /*) ;;
        *) fail "NUMINOUS_HOME must be an absolute path" ;;
    esac
    newline='
'
    case "$NUMINOUS_HOME" in
        *"$newline"*) fail "NUMINOUS_HOME must not contain control characters" ;;
    esac
    if printf '%s' "$NUMINOUS_HOME" | LC_ALL=C grep -q '[[:cntrl:]]'; then
        fail "NUMINOUS_HOME must not contain control characters"
    fi
    while [ "${NUMINOUS_HOME%/}" != "$NUMINOUS_HOME" ]; do
        NUMINOUS_HOME="${NUMINOUS_HOME%/}"
    done
    case "$NUMINOUS_HOME" in
        "" | /) fail "NUMINOUS_HOME must name a dedicated absolute directory" ;;
    esac
    while [ "${NUMINOUS_HOME#//}" != "$NUMINOUS_HOME" ]; do
        NUMINOUS_HOME="${NUMINOUS_HOME#/}"
    done
    case "$NUMINOUS_HOME/" in
        */./* | */../*) fail "NUMINOUS_HOME must not contain . or .. path components" ;;
    esac
    home_physical="$(CDPATH= cd -P "$HOME" 2>/dev/null && pwd)" \
        || fail "HOME is not an accessible directory"
    install_parent="$(dirname "$NUMINOUS_HOME")"
    install_name="$(basename "$NUMINOUS_HOME")"
    install_parent="$(CDPATH= cd -P "$install_parent" 2>/dev/null && pwd)" \
        || fail "the parent directory of NUMINOUS_HOME must already exist"
    NUMINOUS_HOME="$install_parent/$install_name"
    if [ "$NUMINOUS_HOME" = "$home_physical" ] || [ -L "$NUMINOUS_HOME" ]; then
        fail "NUMINOUS_HOME must name a dedicated directory, not HOME or a symbolic link"
    fi
    SOURCE_PATH="$NUMINOUS_HOME/src"
    BINARY_PATH="$NUMINOUS_HOME/bin"
    SOUNDTRACK_PATH="$NUMINOUS_HOME/soundtrack"
    INSTALL_MARKER="$NUMINOUS_HOME/.numinous-install-root"
    DEFAULT_HOME="$home_physical/.numinous"
    if [ -e "$NUMINOUS_HOME" ] && [ ! -d "$NUMINOUS_HOME" ]; then
        fail "NUMINOUS_HOME exists but is not a directory"
    fi
    if [ -d "$NUMINOUS_HOME" ] \
        && ! install_marker_is_valid "$NUMINOUS_HOME" \
        && ! directory_is_empty "$NUMINOUS_HOME"; then
        if [ "$NUMINOUS_HOME" = "$DEFAULT_HOME" ] \
            && legacy_install_is_valid "$NUMINOUS_HOME"; then
            [ "${ADOPT_LEGACY:-0}" -eq 1 ] \
                || fail "a legacy default install needs explicit --adopt-legacy consent"
        else
            fail "NUMINOUS_HOME exists but is not a marked Numinous install root"
        fi
    fi
}

remove_install_root() (
    NUMINOUS_HOME="$1"
    validate_install_root
    [ -e "$NUMINOUS_HOME" ] || exit 0
    if install_marker_is_valid "$NUMINOUS_HOME"; then
        root_kind=marked
        receipt_token="$(install_marker_token "$NUMINOUS_HOME")"
    elif [ "$NUMINOUS_HOME" = "$DEFAULT_HOME" ] \
        && [ "${ADOPT_LEGACY:-0}" -eq 1 ] \
        && legacy_install_is_valid "$NUMINOUS_HOME"; then
        root_kind=legacy
    else
        fail "refusing to remove an unmarked install root: $NUMINOUS_HOME"
    fi
    cd "$install_parent"
    [ ! -L "$install_name" ] \
        || fail "refusing to remove a symbolic-link install root: $NUMINOUS_HOME"
    if [ "$root_kind" = marked ]; then
        install_marker_is_valid "$install_name" \
            || fail "the install root changed during uninstall"
        rm -rf -- "$install_name"
        receipt_dir="$(install_receipt_dir)"
        rm -f -- "$receipt_dir/$receipt_token"
        rmdir -- "$receipt_dir" 2>/dev/null || true
    else
        legacy_install_is_valid "$install_name" \
            || fail "the install root changed during uninstall"
        rm -rf -- "$install_name/src" "$install_name/bin"
        rm -f -- "$install_name/.numinous-install-root"
        rmdir -- "$install_name" \
            || fail "the legacy install root gained unexpected contents during uninstall"
    fi
)

install_source_archive() (
    install_root="$1"
    source_dir="$2"
    binary_dir="$3"
    source_archive="$4"
    install_marker_is_valid "$install_root" \
        || fail "source installation requires a marked install root"
    stage="$(mktemp -d "$install_root/.staging.XXXXXX")" \
        || fail "could not create a source staging directory"
    trap 'rm -rf -- "$stage"' EXIT HUP INT TERM
    tar -xzf "$source_archive" -C "$stage"
    new_tree="$stage/numinous-main"
    [ -d "$new_tree" ] || fail "unexpected source snapshot layout"
    if [ -L "$binary_dir" ]; then
        rm -f -- "$binary_dir"
    elif [ -d "$binary_dir" ]; then
        rm -rf -- "$binary_dir/radio"
    fi
    rm -rf -- "$source_dir"
    mv "$new_tree" "$source_dir"
)

sha256_file() {
    if have sha256sum; then
        sha256sum "$1" | awk '{print $1}'
    elif have shasum; then
        shasum -a 256 "$1" | awk '{print $1}'
    elif have openssl; then
        openssl dgst -sha256 "$1" | awk '{print $NF}'
    else
        fail "SHA-256 verification needs sha256sum, shasum, or openssl"
    fi
}

read_archive_checksum() (
    checksum_path="$1"
    archive_name="$2"
    [ -f "$checksum_path" ] && [ ! -L "$checksum_path" ] \
        || fail "the release checksum sidecar is not an ordinary file"
    [ "$(wc -c <"$checksum_path" | tr -d '[:space:]')" -le 1024 ] \
        || fail "the release checksum sidecar is too large"
    line="$(tr -d '\r\n' <"$checksum_path")"
    hash="${line%%  *}"
    name="${line#*  }"
    [ "$line" = "$hash  $name" ] && [ "$name" = "$archive_name" ] \
        && [ "${#hash}" -eq 64 ] \
        || fail "the release checksum sidecar is malformed or names another archive"
    case "$hash" in *[!0-9a-f]*) fail "the release checksum is not lowercase hexadecimal" ;; esac
    printf '%s' "$hash"
)

read_soundtrack_content_checksum() (
    checksum_path="$1"
    [ -f "$checksum_path" ] && [ ! -L "$checksum_path" ] \
        || fail "the soundtrack content checksum is not an ordinary file"
    [ "$(wc -c <"$checksum_path" | tr -d '[:space:]')" -le 128 ] \
        || fail "the soundtrack content checksum is too large"
    line="$(tr -d '\r\n' <"$checksum_path")"
    hash="${line%%  *}"
    label="${line#*  }"
    [ "$line" = "$hash  $label" ] && [ "$label" = soundtrack-content-v1 ] \
        && [ "${#hash}" -eq 64 ] \
        || fail "the soundtrack content checksum is malformed"
    case "$hash" in *[!0-9a-f]*) fail "the soundtrack content checksum is malformed" ;; esac
    printf '%s' "$hash"
)

assert_archive_checksum() (
    archive_path="$1"
    checksum_path="$2"
    archive_name="$3"
    [ -f "$archive_path" ] && [ ! -L "$archive_path" ] \
        || fail "the release download is not an ordinary archive file"
    expected="$(read_archive_checksum "$checksum_path" "$archive_name")"
    actual="$(sha256_file "$archive_path")"
    [ "$actual" = "$expected" ] \
        || fail "release archive checksum mismatch for $archive_name"
    printf '%s' "$expected"
)

assert_payload_manifest() (
    root="$1"
    [ -d "$root" ] && [ ! -L "$root" ] \
        || fail "the release payload root is not an ordinary directory"
    manifest="$root/MANIFEST.sha256"
    [ -f "$manifest" ] && [ ! -L "$manifest" ] \
        || fail "the release payload manifest is not an ordinary file"
    [ "$(wc -c <"$manifest" | tr -d '[:space:]')" -le 1048576 ] \
        || fail "the release payload manifest is too large"
    linked="$(find "$root" -type l -print -quit)"
    [ -z "$linked" ] || fail "the release payload contains a symbolic link"
    special="$(find "$root" ! -type d ! -type f ! -type l -print -quit)"
    [ -z "$special" ] || fail "the release payload contains a special file"
    listed_count=0
    while IFS= read -r line || [ -n "$line" ]; do
        hash="${line%%  *}"
        relative="${line#*  }"
        [ "$line" = "$hash  $relative" ] && [ "${#hash}" -eq 64 ] \
            || fail "the release payload manifest is malformed"
        case "$hash" in *[!0-9a-f]*) fail "the payload checksum is malformed" ;; esac
        case "$relative" in
            "" | /* | *\\* | *//* | . | .. | ./* | ../* | */. | */.. | */./* | */../*)
                fail "the release payload manifest contains an unsafe path"
                ;;
            *[!A-Za-z0-9._/-]*) fail "the release payload path contains an unsafe byte" ;;
        esac
        candidate="$root/$relative"
        [ -f "$candidate" ] && [ ! -L "$candidate" ] \
            || fail "release payload entry is not an ordinary file: $relative"
        actual="$(sha256_file "$candidate")"
        [ "$actual" = "$hash" ] \
            || fail "release payload checksum mismatch: $relative"
        listed_count=$((listed_count + 1))
    done <"$manifest"
    [ "$listed_count" -gt 0 ] || fail "the release payload manifest is empty"
    inventory="$(mktemp "${TMPDIR:-/tmp}/numinous-inventory.XXXXXX")" \
        || fail "could not stage the release payload inventory"
    trap 'rm -f -- "$inventory" "$inventory.raw"' EXIT HUP INT TERM
    raw_inventory="$inventory.raw"
    find "$root" -type f -print >"$raw_inventory"
    while IFS= read -r candidate || [ -n "$candidate" ]; do
        case "$candidate" in
            "$manifest" | "$root/.archive.sha256") continue ;;
        esac
        printf '%s\n' "$candidate" >>"$inventory"
    done <"$raw_inventory"
    rm -f -- "$raw_inventory"
    while IFS= read -r candidate || [ -n "$candidate" ]; do
        relative="${candidate#"$root"/}"
        grep -Fqx "$(sha256_file "$candidate")  $relative" "$manifest" \
            || fail "release payload contains an unlisted or changed file: $relative"
    done <"$inventory"
    actual_count="$(wc -l <"$inventory" | tr -d '[:space:]')"
    [ "$actual_count" -eq "$listed_count" ] \
        || fail "the release payload inventory differs from its manifest"
    rm -f -- "$inventory"
    trap - EXIT HUP INT TERM
)

soundtrack_content_hash() (
    root="$1"
    manifest="$root/MANIFEST.sha256"
    content="$(mktemp "${TMPDIR:-/tmp}/numinous-soundtrack-content.XXXXXX")" \
        || fail "could not stage the soundtrack content identity"
    trap 'rm -f -- "$content"' EXIT HUP INT TERM
    has_license=0
    mp3_count=0
    while IFS= read -r line || [ -n "$line" ]; do
        hash="${line%%  *}"
        relative="${line#*  }"
        case "$relative" in
            radio/*)
                printf '%s\n' "$line" >>"$content"
                [ "$relative" = radio/ASSET-LICENSE.txt ] && has_license=1
                case "$relative" in *.mp3) mp3_count=$((mp3_count + 1)) ;; esac
                ;;
        esac
    done <"$manifest"
    [ "$has_license" -eq 1 ] && [ "$mp3_count" -gt 0 ] \
        || fail "the soundtrack manifest does not contain licensed audio content"
    sha256_file "$content"
    rm -f -- "$content"
    trap - EXIT HUP INT TERM
)

assert_safe_tar_members() (
    archive_path="$1"
    expected_root="$2"
    members="$(mktemp "${TMPDIR:-/tmp}/numinous-members.XXXXXX")" \
        || fail "could not stage the release archive inventory"
    trap 'rm -f -- "$members"' EXIT HUP INT TERM
    tar -tzf "$archive_path" >"$members" \
        || fail "the release tar archive could not be listed"
    [ -s "$members" ] || fail "the release tar archive is empty"
    while IFS= read -r member || [ -n "$member" ]; do
        case "$member" in
            "$expected_root" | "$expected_root/" | "$expected_root/"*) ;;
            *) fail "the release tar archive escapes its expected root" ;;
        esac
        case "$member" in
            /* | *\\* | *//* | */../* | ../* | */.. | .. | */./* | ./* | */.)
                fail "the release tar archive contains an unsafe member path"
                ;;
            *[!A-Za-z0-9._/-]*)
                fail "the release tar archive member contains an unsafe byte"
                ;;
        esac
    done <"$members"
    rm -f -- "$members"
    trap - EXIT HUP INT TERM
)

install_release_payload() (
    archive_path="$1"
    destination="$2"
    expected_root="$3"
    archive_hash="$4"
    expected_tag="$5"
    expected_kind="$6"
    expected_target="$7"
    expected_content_hash="${8:-}"
    install_marker_is_valid "$NUMINOUS_HOME" \
        || fail "release installation requires a marked install root"
    stage="$(mktemp -d "$NUMINOUS_HOME/.release-stage.XXXXXX")" \
        || fail "could not create a release staging directory"
    trap 'rm -rf -- "$stage"' EXIT HUP INT TERM
    assert_safe_tar_members "$archive_path" "$expected_root"
    tar -xzf "$archive_path" -C "$stage" \
        || fail "could not extract the release archive"
    new_tree="$stage/$expected_root"
    assert_payload_manifest "$new_tree"
    metadata="$new_tree/RELEASE.json"
    grep -Fqx '  "schema": "numinous.release",' "$metadata" \
        && grep -Fqx '  "schemaVersion": 1,' "$metadata" \
        && grep -Fqx "  \"tag\": \"$expected_tag\"," "$metadata" \
        && grep -Fqx "  \"kind\": \"$expected_kind\"," "$metadata" \
        && grep -Fqx "  \"target\": \"$expected_target\"," "$metadata" \
        || fail "the release metadata does not match the requested payload"
    if [ -n "$expected_content_hash" ]; then
        [ "$(soundtrack_content_hash "$new_tree")" = "$expected_content_hash" ] \
            || fail "the soundtrack content checksum does not match the verified payload"
    fi
    printf '%s\n' "$archive_hash" >"$new_tree/.archive.sha256"
    rm -rf -- "$destination"
    mv "$new_tree" "$destination"
)

latest_release_tag() (
    metadata="$(mktemp "${TMPDIR:-/tmp}/numinous-releases.XXXXXX")" \
        || fail "could not stage release metadata"
    trap 'rm -f -- "$metadata"' EXIT HUP INT TERM
    fetch "$REPO_API_URL/releases?per_page=20" "$metadata"
    tag="$(sed -n 's/^[[:space:]]*"tag_name": "\(v[0-9][0-9A-Za-z.-]*\)",$/\1/p' "$metadata" | sed -n '1p')"
    printf '%s' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$' \
        || fail "no safe published Numinous release is available; use --source to build main"
    printf '%s' "$tag"
)

release_target() {
    machine="$(uname -m)"
    case "$os:$machine" in
        linux:x86_64 | linux:amd64) printf '%s' x86_64-unknown-linux-gnu ;;
        macos:x86_64) printf '%s' x86_64-apple-darwin ;;
        macos:arm64 | macos:aarch64) printf '%s' aarch64-apple-darwin ;;
        *) fail "no published release supports $os $machine; re-run with --source" ;;
    esac
}

copy_release_file() {
    provided_path="$1"
    url="$2"
    destination="$3"
    description="$4"
    if [ -n "$provided_path" ]; then
        case "$provided_path" in
            /*) source_path="$provided_path" ;;
            *) source_path="$INVOCATION_DIR/$provided_path" ;;
        esac
        [ -f "$source_path" ] && [ ! -L "$source_path" ] \
            || fail "$description fixture is not an ordinary file"
        cp "$source_path" "$destination"
    else
        say "Downloading $description"
        fetch "$url" "$destination"
    fi
}

installed_soundtrack_is_current() (
    expected_content_hash="$1"
    [ -d "$SOUNDTRACK_PATH" ] && [ ! -L "$SOUNDTRACK_PATH" ] || return 1
    receipt="$SOUNDTRACK_PATH/.archive.sha256"
    [ -f "$receipt" ] && [ ! -L "$receipt" ] || return 1
    receipt_hash="$(tr -d '\r\n' <"$receipt")"
    [ "${#receipt_hash}" -eq 64 ] || return 1
    case "$receipt_hash" in *[!0-9a-f]*) return 1 ;; esac
    assert_payload_manifest "$SOUNDTRACK_PATH" >/dev/null 2>&1 || return 1
    [ "$(soundtrack_content_hash "$SOUNDTRACK_PATH")" = "$expected_content_hash" ]
)

install_latest_release() (
    if [ -n "$RELEASE_TAG" ]; then tag="$RELEASE_TAG"; else tag="$(latest_release_tag)"; fi
    printf '%s' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$' \
        || fail "the requested release tag is unsafe"
    if { [ -n "$RELEASE_ARCHIVE" ] && [ -z "$RELEASE_CHECKSUM" ]; } \
        || { [ -z "$RELEASE_ARCHIVE" ] && [ -n "$RELEASE_CHECKSUM" ]; }; then
        fail "local release fixtures require matching archive and checksum paths"
    fi
    if { [ -n "$SOUNDTRACK_ARCHIVE" ] || [ -n "$SOUNDTRACK_CHECKSUM" ] \
        || [ -n "$SOUNDTRACK_CONTENT_CHECKSUM" ]; } \
        && { [ -z "$SOUNDTRACK_ARCHIVE" ] || [ -z "$SOUNDTRACK_CHECKSUM" ] \
        || [ -z "$SOUNDTRACK_CONTENT_CHECKSUM" ]; }; then
        fail "local soundtrack fixtures require matching archive and checksum paths"
    fi
    target="$(release_target)"
    payload_root="numinous-$tag-$target"
    payload_name="$payload_root.tar.gz"
    soundtrack_root="numinous-$tag-soundtrack"
    soundtrack_name="$soundtrack_root.tar.gz"
    soundtrack_content_name="$soundtrack_name.content.sha256"
    stage="$(mktemp -d "$NUMINOUS_HOME/.download.XXXXXX")" \
        || fail "could not create a release download directory"
    trap 'rm -rf -- "$stage"' EXIT HUP INT TERM
    release_base="$REPO_URL/releases/download/$tag"
    payload_path="$stage/$payload_name"
    payload_checksum_path="$stage/$payload_name.sha256"
    copy_release_file "$RELEASE_ARCHIVE" "$release_base/$payload_name" \
        "$payload_path" "the $os release payload"
    copy_release_file "$RELEASE_CHECKSUM" "$release_base/$payload_name.sha256" \
        "$payload_checksum_path" "the $os payload checksum"
    payload_hash="$(assert_archive_checksum "$payload_path" "$payload_checksum_path" "$payload_name")"

    soundtrack_checksum_path="$stage/$soundtrack_name.sha256"
    copy_release_file "$SOUNDTRACK_CHECKSUM" "$release_base/$soundtrack_name.sha256" \
        "$soundtrack_checksum_path" "the soundtrack checksum"
    soundtrack_hash="$(read_archive_checksum "$soundtrack_checksum_path" "$soundtrack_name")"
    soundtrack_content_path="$stage/$soundtrack_content_name"
    copy_release_file "$SOUNDTRACK_CONTENT_CHECKSUM" \
        "$release_base/$soundtrack_content_name" "$soundtrack_content_path" \
        "the soundtrack content checksum"
    soundtrack_content_hash="$(read_soundtrack_content_checksum "$soundtrack_content_path")"

    rm -rf -- "$BINARY_PATH/radio"
    install_release_payload "$payload_path" "$SOURCE_PATH" "$payload_root" \
        "$payload_hash" "$tag" binaries "$target"
    if installed_soundtrack_is_current "$soundtrack_content_hash"; then
        say "The verified built-in soundtrack is already current."
    else
        soundtrack_path="$stage/$soundtrack_name"
        copy_release_file "$SOUNDTRACK_ARCHIVE" "$release_base/$soundtrack_name" \
            "$soundtrack_path" "the built-in soundtrack"
        assert_archive_checksum "$soundtrack_path" "$soundtrack_checksum_path" \
            "$soundtrack_name" >/dev/null
        install_release_payload "$soundtrack_path" "$SOUNDTRACK_PATH" \
            "$soundtrack_root" "$soundtrack_hash" "$tag" soundtrack all \
            "$soundtrack_content_hash"
    fi
    printf '%s\n' "$tag" >"$NUMINOUS_HOME/.installed-release"
)

verify_installed_cli() (
    binary_dir="$1"
    previous_path="$2"
    PATH="$binary_dir:$previous_path"
    export PATH
    resolved_cli="$(command -v numinous 2>/dev/null || true)"
    installed_cli="$binary_dir/numinous"
    [ "$resolved_cli" = "$installed_cli" ] \
        || fail "PATH verification resolved numinous to $resolved_cli instead of $installed_cli"
    "$installed_cli" --version \
        || fail "the installed CLI did not pass its absolute-path version check"
)

strip_path_line() (
    profile="$1"
    [ -f "$profile" ] || return 0
    if grep -Fq "$INSTALLER_NOTE" "$profile"; then
        :
    else
        status="$?"
        [ "$status" -eq 1 ] && return 0
        fail "could not read the shell profile"
    fi
    tmp="$(mktemp "${TMPDIR:-/tmp}/numinous-profile.XXXXXX")" \
        || fail "could not stage the shell profile update"
    trap 'rm -f -- "$tmp"' EXIT HUP INT TERM
    if grep -Fv "$INSTALLER_NOTE" "$profile" >"$tmp"; then
        :
    else
        status="$?"
        [ "$status" -eq 1 ] || fail "could not read the shell profile"
    fi
    cat "$tmp" >"$profile" || fail "could not update the shell profile"
    rm -f -- "$tmp"
    trap - EXIT HUP INT TERM
)

add_path_line() {
    profile="$1"
    line="$2"
    strip_path_line "$profile"
    printf '\n%s\n' "$line" >>"$profile"
}

run_self_test() {
    have tar || fail "installer self-test requires tar"
    test_base="$(mktemp -d "${TMPDIR:-/tmp}/numinous-installer-test.XXXXXX")" \
        || fail "could not create the installer self-test directory"
    trap 'rm -rf -- "$test_base"' EXIT HUP INT TERM
    HOME="$test_base/home"
    export HOME
    mkdir "$HOME"
    chmod 700 "$HOME" 2>/dev/null || self_test_without_posix_modes

    content_a="$test_base/content-a"
    content_b="$test_base/content-b"
    mkdir "$content_a" "$content_b"
    radio_license_hash="$(printf '%064d' 1)"
    radio_track_hash="$(printf '%064d' 2)"
    release_hash_a="$(printf '%064d' 3)"
    release_hash_b="$(printf '%064d' 4)"
    {
        printf '%s  RELEASE.json\n' "$release_hash_a"
        printf '%s  radio/ASSET-LICENSE.txt\n' "$radio_license_hash"
        printf '%s  radio/test-001.mp3\n' "$radio_track_hash"
    } >"$content_a/MANIFEST.sha256"
    {
        printf '%s  RELEASE.json\n' "$release_hash_b"
        printf '%s  radio/ASSET-LICENSE.txt\n' "$radio_license_hash"
        printf '%s  radio/test-001.mp3\n' "$radio_track_hash"
    } >"$content_b/MANIFEST.sha256"
    content_hash="$(soundtrack_content_hash "$content_a")"
    [ "$(soundtrack_content_hash "$content_b")" = "$content_hash" ] \
        || fail "soundtrack self-test: release metadata changed the content identity"
    printf '%s  soundtrack-content-v1\n' "$content_hash" \
        >"$test_base/soundtrack.content.sha256"
    [ "$(read_soundtrack_content_checksum "$test_base/soundtrack.content.sha256")" \
        = "$content_hash" ] \
        || fail "soundtrack self-test: the content checksum did not round-trip"

    printf '%s\n' fixture >"$test_base/relative-fixture"
    mkdir "$test_base/fixture-work"
    (
        INVOCATION_DIR="$test_base"
        cd "$test_base/fixture-work"
        copy_release_file relative-fixture ignored-url copied-fixture \
            "the relative release payload"
        cmp copied-fixture "$test_base/relative-fixture"
    ) || fail "fixture self-test: a relative path stopped resolving after a directory change"

    if (NUMINOUS_HOME="$HOME"; validate_install_root) >/dev/null 2>&1; then
        fail "root self-test: HOME was accepted as an install root"
    fi

    unmarked="$test_base/unmarked"
    mkdir "$unmarked"
    printf '%s\n' keep >"$unmarked/keep.txt"
    printf '%s\n' 'not a marker' >"$unmarked/.numinous-install-root"
    if remove_install_root "$unmarked" >/dev/null 2>&1; then
        fail "uninstall self-test: an unmarked root was accepted"
    fi
    [ -d "$unmarked" ] || fail "uninstall self-test: an unmarked root was removed"

    legacy_update="$HOME/.numinous"
    mkdir -p "$legacy_update/src" "$legacy_update/bin"
    printf '%s\n' '[workspace]' >"$legacy_update/src/Cargo.toml"
    for binary in numinous numinous-app numinous-mcp; do
        printf '%s\n' binary >"$legacy_update/bin/$binary"
    done
    if (NUMINOUS_HOME="$legacy_update"; validate_install_root) >/dev/null 2>&1; then
        fail "root self-test: a legacy default install migrated without explicit consent"
    fi
    (ADOPT_LEGACY=1; NUMINOUS_HOME="$legacy_update"; validate_install_root) \
        || fail "root self-test: the exact legacy install shape could not migrate"
    printf '%s\n' keep >"$legacy_update/unexpected.txt"
    if (ADOPT_LEGACY=1; NUMINOUS_HOME="$legacy_update"; validate_install_root) \
        >/dev/null 2>&1; then
        fail "root self-test: a legacy root with unexpected contents was accepted"
    fi
    rm -f -- "$legacy_update/unexpected.txt"
    rm -rf -- "$legacy_update"

    legacy_uninstall="$HOME/.numinous"
    mkdir -p "$legacy_uninstall/src" "$legacy_uninstall/bin"
    printf '%s\n' '[workspace]' >"$legacy_uninstall/src/Cargo.toml"
    for binary in numinous numinous-app numinous-mcp; do
        printf '%s\n' binary >"$legacy_uninstall/bin/$binary"
    done
    printf '%s\n' "$LEGACY_INSTALL_MARKER_TEXT" \
        >"$legacy_uninstall/.numinous-install-root"
    if remove_install_root "$legacy_uninstall" >/dev/null 2>&1; then
        fail "uninstall self-test: a legacy default install was removed without explicit consent"
    fi
    [ -d "$legacy_uninstall" ] \
        || fail "uninstall self-test: rejected legacy removal changed the root"
    (ADOPT_LEGACY=1; remove_install_root "$legacy_uninstall")
    [ ! -e "$legacy_uninstall" ] \
        || fail "uninstall self-test: the exact legacy install was retained"

    forged="$HOME/.numinous"
    mkdir "$forged"
    printf '%s\n' "$LEGACY_INSTALL_MARKER_TEXT" >"$forged/.numinous-install-root"
    printf '%s\n' keep >"$forged/keep.txt"
    if remove_install_root "$forged" >/dev/null 2>&1; then
        fail "uninstall self-test: a forged public marker was accepted"
    fi
    [ -f "$forged/keep.txt" ] \
        || fail "uninstall self-test: a forged public marker removed unrelated data"

    marked="$test_base/marked"
    mkdir "$marked"
    claim_install_root "$marked"
    printf '%s\n' keep >"$test_base/adjacent.txt"
    remove_install_root "$marked"
    [ ! -e "$marked" ] && [ -f "$test_base/adjacent.txt" ] \
        || fail "uninstall self-test: marked-root removal crossed its boundary"

    source_root="$test_base/source-root"
    source_dir="$source_root/src"
    binary_dir="$source_root/bin"
    mkdir "$source_root"
    claim_install_root "$source_root"
    mkdir -p "$source_dir/.git" "$source_dir/target"
    printf '%s\n' 'alternate origin' >"$source_dir/.git/config"
    printf '%s\n' untrusted >"$source_dir/untrusted.txt"
    printf '%s\n' 'untrusted cache' >"$source_dir/target/cached.txt"
    mkdir -p "$test_base/source-outside/radio"
    printf '%s\n' keep >"$test_base/source-outside/radio/keep.txt"
    ln -s "$test_base/source-outside" "$binary_dir"
    mkdir -p "$test_base/package/numinous-main"
    printf '%s\n' trusted >"$test_base/package/numinous-main/trusted.txt"
    (cd "$test_base/package" && tar -czf "$test_base/trusted.tar.gz" numinous-main)
    install_source_archive "$source_root" "$source_dir" "$binary_dir" \
        "$test_base/trusted.tar.gz"
    [ -f "$source_dir/trusted.txt" ] \
        && [ ! -e "$source_dir/untrusted.txt" ] \
        && [ ! -e "$source_dir/target/cached.txt" ] \
        && [ -f "$test_base/source-outside/radio/keep.txt" ] \
        || fail "provenance self-test: old source or build cache influenced the update"

    mkdir "$test_base/installed-bin" "$test_base/stale-bin"
    printf '%s\n' '#!/bin/sh' 'exit 0' >"$test_base/installed-bin/numinous"
    printf '%s\n' '#!/bin/sh' 'exit 99' >"$test_base/stale-bin/numinous"
    chmod +x "$test_base/installed-bin/numinous" "$test_base/stale-bin/numinous"
    profile="$HOME/.profile"
    printf '%s\n' \
        "export PATH=\"$test_base/stale-bin:\$PATH\" # $INSTALLER_NOTE" \
        "export PATH=\"$test_base/stale-bin:\$PATH\"" >"$profile"
    chmod 600 "$profile" 2>/dev/null || self_test_without_posix_modes
    quoted_test_bin="$(posix_quote "$test_base/installed-bin")"
    test_path_line="export PATH=$quoted_test_bin:\$PATH # $INSTALLER_NOTE"
    add_path_line "$profile" "$test_path_line"
    [ "$(grep -Fc "$INSTALLER_NOTE" "$profile")" = 1 ] \
        || fail "PATH self-test: the installer-owned line was duplicated"
    set -- $(stat_owner_mode_identity "$profile")
    [ "$#" = 3 ] && mode_is "$2" 600 \
        || fail "PATH self-test: profile refresh changed its access mode"
    resolved_from_profile="$(. "$profile"; command -v numinous)"
    [ "$resolved_from_profile" = "$test_base/installed-bin/numinous" ] \
        || fail "PATH self-test: the refreshed profile retained stale precedence"
    linked_profile="$HOME/.bashrc"
    linked_target="$HOME/managed-profile"
    cp "$profile" "$linked_target"
    if ln -s "$linked_target" "$linked_profile" 2>/dev/null \
        && [ -L "$linked_profile" ]; then
        add_path_line "$linked_profile" "$test_path_line"
        [ -L "$linked_profile" ] \
            || fail "PATH self-test: profile refresh replaced a symbolic link"
        [ "$(grep -Fc "$INSTALLER_NOTE" "$linked_target")" = 1 ] \
            || fail "PATH self-test: profile refresh missed the symbolic-link target"
    else
        rm -f -- "$linked_profile"
    fi
    verify_installed_cli "$test_base/installed-bin" "$test_base/stale-bin:$PATH" \
        || fail "PATH self-test: a stale earlier command defeated verified precedence"

    rm -rf -- "$test_base"
    trap - EXIT HUP INT TERM
    say "POSIX installer root, uninstall, and provenance checks: pass."
}

usage() {
    say "Numinous installer (macOS and Linux)."
    say ""
    say "  install.sh                  install or update Numinous"
    say "  install.sh --uninstall      remove ~/.numinous and the PATH lines it added"
    say "  install.sh --no-modify-path install without editing any shell profile"
    say "  install.sh --adopt-legacy   explicitly migrate an older default-root install"
    say "  install.sh --source         build the current main branch from source"
    say ""
    say "NUMINOUS_HOME overrides the install root (default ~/.numinous)."
    say "Play history in ~/.numinous-journey and friends is never touched."
}

UNINSTALL=0
MODIFY_PATH=1
ADOPT_LEGACY=0
SOURCE_MODE=0
SELF_TEST=0
RELEASE_ARCHIVE=''
RELEASE_CHECKSUM=''
SOUNDTRACK_ARCHIVE=''
SOUNDTRACK_CHECKSUM=''
SOUNDTRACK_CONTENT_CHECKSUM=''
RELEASE_TAG=''
WAIT_FOR_PID=0
DELETE_INSTALLER=''
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall) UNINSTALL=1 ;;
        --no-modify-path) MODIFY_PATH=0 ;;
        --adopt-legacy) ADOPT_LEGACY=1 ;;
        --source) SOURCE_MODE=1 ;;
        --self-test) SELF_TEST=1 ;;
        --release-archive | --release-checksum | --soundtrack-archive | \
            --soundtrack-checksum | --soundtrack-content-checksum | \
            --release-tag | --wait-for-pid | \
            --delete-installer)
            option="$1"
            shift
            [ $# -gt 0 ] || fail "$option needs a value"
            case "$option" in
                --release-archive) RELEASE_ARCHIVE="$1" ;;
                --release-checksum) RELEASE_CHECKSUM="$1" ;;
                --soundtrack-archive) SOUNDTRACK_ARCHIVE="$1" ;;
                --soundtrack-checksum) SOUNDTRACK_CHECKSUM="$1" ;;
                --soundtrack-content-checksum) SOUNDTRACK_CONTENT_CHECKSUM="$1" ;;
                --release-tag) RELEASE_TAG="$1" ;;
                --wait-for-pid) WAIT_FOR_PID="$1" ;;
                --delete-installer) DELETE_INSTALLER="$1" ;;
            esac
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *) fail "unknown option '$1' (try --help)" ;;
    esac
    shift
done

if [ "$SELF_TEST" -eq 1 ]; then
    run_self_test
    exit 0
fi

if [ "$SOURCE_MODE" -eq 1 ] \
    && { [ -n "$RELEASE_ARCHIVE" ] || [ -n "$RELEASE_CHECKSUM" ] \
        || [ -n "$SOUNDTRACK_ARCHIVE" ] || [ -n "$SOUNDTRACK_CHECKSUM" ] \
        || [ -n "$SOUNDTRACK_CONTENT_CHECKSUM" ] \
        || [ -n "$RELEASE_TAG" ]; }; then
    fail "--source cannot be combined with release fixture options"
fi
case "$WAIT_FOR_PID" in
    '' | *[!0-9]*) fail "--wait-for-pid needs a non-negative process id" ;;
esac
[ "$WAIT_FOR_PID" != "$$" ] || fail "the update helper cannot wait for itself"
if [ "$WAIT_FOR_PID" -gt 0 ]; then
    say "Waiting for the running Numinous command to close before updating."
    while kill -0 "$WAIT_FOR_PID" 2>/dev/null; do sleep 1; done
fi

cleanup_update_installer() {
    [ -n "$DELETE_INSTALLER" ] || return 0
    case "$DELETE_INSTALLER" in /*) ;; *) return 0 ;; esac
    [ -f "$DELETE_INSTALLER" ] && [ ! -L "$DELETE_INSTALLER" ] || return 0
    delete_parent="$(CDPATH= cd -P "$(dirname "$DELETE_INSTALLER")" 2>/dev/null && pwd)" \
        || return 0
    temp_parent="$(CDPATH= cd -P "${TMPDIR:-/tmp}" 2>/dev/null && pwd)" || return 0
    [ "$delete_parent" = "$temp_parent" ] || return 0
    delete_name="$(basename "$DELETE_INSTALLER")"
    printf '%s' "$delete_name" \
        | grep -Eq '^numinous-update-[0-9a-f]{32}\.sh$' || return 0
    rm -f -- "$DELETE_INSTALLER"
}
trap cleanup_update_installer EXIT HUP INT TERM

validate_install_root

case "$(uname -s)" in
    Darwin) os=macos ;;
    Linux) os=linux ;;
    MINGW* | MSYS* | CYGWIN*)
        fail "this looks like Windows. Use the PowerShell installer instead:
  irm ${INSTALL_PS1_URL} | iex"
        ;;
    *) fail "unsupported system '$(uname -s)'; Numinous builds on macOS, Linux, and Windows" ;;
esac

# The line this installer appends to shell profiles. The note at its end is
# the removal key: --uninstall deletes exactly the lines carrying the note,
# never a user's own PATH edits. Re-runs replace that owned line so changing
# NUMINOUS_HOME cannot leave a stale Numinous binary ahead of this install.
if [ "$NUMINOUS_HOME" = "$DEFAULT_HOME" ]; then
    path_line="export PATH=\"\$HOME/.numinous/bin:\$PATH\" # $INSTALLER_NOTE"
else
    quoted_bin_dir="$(posix_quote "$BINARY_PATH")"
    path_line="export PATH=$quoted_bin_dir:\$PATH # $INSTALLER_NOTE"
fi

if [ "$UNINSTALL" -eq 1 ]; then
    remove_install_root "$NUMINOUS_HOME"
    for profile in "$HOME/.profile" "$HOME/.bash_profile" "$HOME/.bashrc" \
        "$HOME/.zprofile" "$HOME/.zshrc"; do
        strip_path_line "$profile"
    done
    rm -f "$HOME/.config/fish/conf.d/numinous.fish"
    say "Numinous is uninstalled: $NUMINOUS_HOME is gone and the PATH lines are removed."
    say "Your play history stays: ~/.numinous-journey, ~/.numinous-scores, ~/.numinous-cairn."
    exit 0
fi

cd "$install_parent"
if [ ! -e "$install_name" ]; then
    mkdir "$install_name"
fi
if [ ! -d "$install_name" ] || [ -L "$install_name" ]; then
    fail "NUMINOUS_HOME changed while the installer was starting"
fi
cd -P "$install_name"
if [ "$(pwd -P)" != "$NUMINOUS_HOME" ]; then
    fail "NUMINOUS_HOME changed while the installer was starting"
fi
if ! install_marker_is_valid "$NUMINOUS_HOME"; then
    claim_install_root "$NUMINOUS_HOME"
fi
SRC_DIR=src
BIN_DIR=bin

# A downloader is needed for rustup and for the no-git source fallback.
if have curl; then
    fetch() { curl -fsSL "$1" -o "$2"; }
elif have wget; then
    fetch() { wget -qO "$2" "$1"; }
else
    fail "neither curl nor wget is installed; install one and re-run"
fi

# Building main from source remains available as an explicit fallback.
if [ "$SOURCE_MODE" -eq 1 ]; then
    # A C toolchain is needed to link the Rust build. On macOS the cc on PATH is
    # a shim, so ask xcode-select whether the real tools are installed.
    if [ "$os" = macos ]; then
        if ! xcode-select -p >/dev/null 2>&1; then
            fail "the Xcode command line tools are not installed. Install them first:
  xcode-select --install
then re-run this installer"
        fi
    elif ! have cc && ! have gcc && ! have clang; then
        fail "no C compiler found. Install one first, then re-run this installer.
  Debian/Ubuntu: sudo apt-get install -y build-essential
  Fedora:        sudo dnf install -y gcc
  Arch:          sudo pacman -S --needed base-devel"
    fi

    # The audio, window, and controller builds need ALSA, xkbcommon, and libudev
    # headers on Linux (the same packages CI installs).
    if [ "$os" = linux ]; then
        if ! have pkg-config || ! pkg-config --exists alsa xkbcommon libudev 2>/dev/null; then
            fail "the build needs pkg-config plus the ALSA, xkbcommon, and libudev headers. Install them, then re-run.
  Debian/Ubuntu: sudo apt-get install -y pkg-config libasound2-dev libxkbcommon-dev libudev-dev
  Fedora:        sudo dnf install -y pkgconf-pkg-config alsa-lib-devel libxkbcommon-devel systemd-devel
  Arch:          sudo pacman -S --needed pkgconf alsa-lib libxkbcommon systemd-libs
  openSUSE:      sudo zypper install pkg-config alsa-devel libxkbcommon-devel libudev-devel"
        fi
    fi

    # Rust. Prefer rustup, which honors the exact toolchain pinned in
    # rust-toolchain.toml. Reuse an existing ~/.cargo install when present.
    if [ -d "$HOME/.cargo/bin" ]; then
        PATH="$HOME/.cargo/bin:$PATH"
    fi
    if ! have cargo; then
        say "Rust is not installed yet. Installing it with rustup (https://rustup.rs)."
        rustup_init="$(mktemp)"
        fetch "https://sh.rustup.rs" "$rustup_init"
        if [ "$MODIFY_PATH" -eq 1 ]; then
            sh "$rustup_init" -y --default-toolchain none </dev/null
        else
            sh "$rustup_init" -y --default-toolchain none --no-modify-path </dev/null
        fi
        rm -f "$rustup_init"
        PATH="$HOME/.cargo/bin:$PATH"
        have cargo || fail "rustup finished but cargo is still missing; open a new shell and re-run"
    fi
    if ! have rustup; then
        # A distro cargo without rustup cannot honor the pinned toolchain file,
        # so accept it only if it meets the workspace MSRV in Cargo.toml.
        cargo_minor="$(cargo --version 2>/dev/null | sed -n 's/^cargo 1\.\([0-9][0-9]*\).*/\1/p')"
        if [ -z "$cargo_minor" ] || [ "$cargo_minor" -lt 88 ]; then
            fail "this cargo is older than the minimum supported Rust (1.88) and rustup is absent.
Install rustup from https://rustup.rs and re-run this installer"
        fi
        say "note: using the system cargo without rustup; the pinned toolchain file is ignored."
    fi

    # Replace the source from the fixed snapshot on every run. Existing
    # repository configuration, untracked files, and build caches cannot
    # influence an update.
    have tar || fail "tar is required to extract the trusted source snapshot"
    source_archive="$(mktemp "$NUMINOUS_HOME/.source.XXXXXX")" \
        || fail "could not create a source download file"
    say "Downloading the trusted source snapshot from $REPO_URL"
    fetch "$SNAPSHOT_URL" "$source_archive"
    install_source_archive "$NUMINOUS_HOME" "$SOURCE_PATH" "$BINARY_PATH" "$source_archive"
    rm -f -- "$source_archive"

    if have rustup; then
        # Install the pinned toolchain up front so the build step is only a
        # build. Older rustup releases need the toolchain named; current cargo
        # installs it on demand anyway, so a failure here is not fatal.
        (cd "$SRC_DIR" && rustup toolchain install) || true
    fi

    say "Building the release binaries (the first build takes several minutes)."
    (cd "$SRC_DIR" && cargo build --release --locked \
        --bin numinous --bin numinous-app --bin numinous-mcp)
    BINARY_SOURCE="$SRC_DIR/target/release"
    RADIO_SOURCE="$SOURCE_PATH/assets/radio"
else
    have tar || fail "tar is required to extract verified release archives"
    install_latest_release
    BINARY_SOURCE="$SRC_DIR/bin"
    RADIO_SOURCE="$SOUNDTRACK_PATH/radio"
fi

mkdir -p "$BIN_DIR"
for binary in numinous numinous-app numinous-mcp; do
    binary_stage="$(mktemp "$BIN_DIR/.numinous-$binary.XXXXXX")" \
        || fail "could not create a binary staging file"
    if install -m 755 "$BINARY_SOURCE/$binary" "$binary_stage" \
        && mv -f -- "$binary_stage" "$BIN_DIR/$binary"; then
        :
    else
        rm -f -- "$binary_stage"
        fail "could not publish $binary"
    fi
done
# The app finds the built-in radio next to its executable.
ln -sfn "$RADIO_SOURCE" "$BIN_DIR/radio"

if [ "$MODIFY_PATH" -eq 1 ]; then
    add_path_line "$HOME/.profile" "$path_line"
    # A login bash reads .bash_profile instead of .profile when it exists.
    for profile in "$HOME/.bash_profile" "$HOME/.bashrc"; do
        if [ -f "$profile" ]; then
            add_path_line "$profile" "$path_line"
        fi
    done
    if [ -f "$HOME/.zshrc" ] || [ "${SHELL##*/}" = "zsh" ]; then
        add_path_line "$HOME/.zshrc" "$path_line"
    fi
    if [ -d "$HOME/.config/fish" ]; then
        mkdir -p "$HOME/.config/fish/conf.d"
        quoted_fish_bin="$(fish_quote "$BINARY_PATH")"
        printf '%s\n' \
            "# added by the Numinous installer" \
            "if test -d $quoted_fish_bin" \
            "    fish_add_path --prepend $quoted_fish_bin" \
            "end" >"$HOME/.config/fish/conf.d/numinous.fish"
    fi
fi

verify_installed_cli "$BINARY_PATH" "$PATH"
PATH="$BINARY_PATH:$PATH"
export PATH

say ""
say "Numinous is installed."
say ""
say "  numinous-app     the window: rooms, sound, games, the radio"
say "  numinous         the same world, live in the terminal"
say ""
say "Digital minds connect over MCP:"
say "  claude mcp add numinous -- $BINARY_PATH/numinous-mcp"
say ""
if [ "$MODIFY_PATH" -eq 1 ]; then
    say "PATH was updated. Open a new terminal, then launch the verified app path:"
    say "  $BINARY_PATH/numinous-app"
else
    say "PATH was not modified. Add this yourself, or run the binaries by full path:"
    say "  $path_line"
fi
say ""
say "Read PLAY.md first if you read anything: $SOURCE_PATH/PLAY.md"
if [ "$SOURCE_MODE" -eq 1 ]; then
    say "This source build follows main. Re-run with --source to update it."
else
    installed_release="$(cat "$NUMINOUS_HOME/.installed-release")"
    say "Installed release: $installed_release"
    say "Update any time with: numinous update"
fi
say "Uninstall with --uninstall."

cleanup_update_installer
trap - EXIT HUP INT TERM
