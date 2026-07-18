#!/bin/sh
# Numinous installer for macOS and Linux. One line to play:
#
#   curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
#
# What it does, in order: checks the tools this machine needs (and says exactly
# how to get any that are missing), installs Rust through rustup if cargo is
# absent, fetches a trusted source snapshot into ~/.numinous/src, builds the
# release binaries, puts numinous,
# numinous-app, and numinous-mcp in ~/.numinous/bin, links the built-in radio
# next to them, and adds that directory to PATH.
#
# Re-run it any time to update. Remove everything it installed with:
#
#   curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh -s -- --uninstall
#
# Uninstalling never touches play history: ~/.numinous-journey,
# ~/.numinous-scores, and ~/.numinous-cairn stay yours.
#
# Options: --uninstall, --no-modify-path, --adopt-legacy, --self-test, --help.
# Set NUMINOUS_HOME to install somewhere other than ~/.numinous.
set -eu

REPO="blisspixel/numinous"
REPO_URL="https://github.com/${REPO}"
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
    say ""
    say "NUMINOUS_HOME overrides the install root (default ~/.numinous)."
    say "Play history in ~/.numinous-journey and friends is never touched."
}

UNINSTALL=0
MODIFY_PATH=1
ADOPT_LEGACY=0
SELF_TEST=0
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall) UNINSTALL=1 ;;
        --no-modify-path) MODIFY_PATH=0 ;;
        --adopt-legacy) ADOPT_LEGACY=1 ;;
        --self-test) SELF_TEST=1 ;;
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
    # A distro cargo without rustup cannot honor the pinned toolchain file, so
    # accept it only if it meets the workspace MSRV in Cargo.toml.
    cargo_minor="$(cargo --version 2>/dev/null | sed -n 's/^cargo 1\.\([0-9][0-9]*\).*/\1/p')"
    if [ -z "$cargo_minor" ] || [ "$cargo_minor" -lt 88 ]; then
        fail "this cargo is older than the minimum supported Rust (1.88) and rustup is absent.
Install rustup from https://rustup.rs and re-run this installer"
    fi
    say "note: using the system cargo without rustup; the pinned toolchain file is ignored."
fi

# Replace the source from the fixed snapshot on every run. Existing repository
# configuration, untracked files, and build caches never influence an update.
have tar || fail "tar is required to extract the trusted source snapshot"
source_archive="$(mktemp "$NUMINOUS_HOME/.source.XXXXXX")" \
    || fail "could not create a source download file"
trap 'rm -f -- "$source_archive"' EXIT HUP INT TERM
say "Downloading the trusted source snapshot from $REPO_URL"
fetch "$SNAPSHOT_URL" "$source_archive"
install_source_archive "$NUMINOUS_HOME" "$SOURCE_PATH" "$BINARY_PATH" "$source_archive"
rm -f -- "$source_archive"
trap - EXIT HUP INT TERM

if have rustup; then
    # Install the pinned toolchain up front so the build step is only a build.
    # Older rustup releases need the toolchain named; current cargo installs it
    # on demand anyway, so a failure here is not fatal.
    (cd "$SRC_DIR" && rustup toolchain install) || true
fi

say "Building the release binaries (the first build takes several minutes)."
(cd "$SRC_DIR" && cargo build --release --locked \
    --bin numinous --bin numinous-app --bin numinous-mcp)

mkdir -p "$BIN_DIR"
for binary in numinous numinous-app numinous-mcp; do
    binary_stage="$(mktemp "$BIN_DIR/.numinous-$binary.XXXXXX")" \
        || fail "could not create a binary staging file"
    if install -m 755 "$SRC_DIR/target/release/$binary" "$binary_stage" \
        && mv -f -- "$binary_stage" "$BIN_DIR/$binary"; then
        :
    else
        rm -f -- "$binary_stage"
        fail "could not publish $binary"
    fi
done
# The app finds the built-in radio next to its executable.
ln -sfn "$SOURCE_PATH/assets/radio" "$BIN_DIR/radio"

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
say "Update any time by re-running this installer. Uninstall with --uninstall."
