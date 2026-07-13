#!/bin/sh
# Numinous installer for macOS and Linux. One line to play:
#
#   curl -fsSL https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.sh | sh
#
# What it does, in order: checks the tools this machine needs (and says exactly
# how to get any that are missing), installs Rust through rustup if cargo is
# absent, fetches the source into ~/.numinous/src (git when available, a
# snapshot download otherwise), builds the release binaries, puts numinous,
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
# Options: --uninstall, --no-modify-path, --help.
# Set NUMINOUS_HOME to install somewhere other than ~/.numinous.
set -eu

REPO="blisspixel/numinous"
REPO_URL="https://github.com/${REPO}"
SNAPSHOT_URL="https://codeload.github.com/${REPO}/tar.gz/refs/heads/main"
INSTALL_SH_URL="https://raw.githubusercontent.com/${REPO}/main/scripts/install.sh"
INSTALL_PS1_URL="https://raw.githubusercontent.com/${REPO}/main/scripts/install.ps1"
NUMINOUS_HOME="${NUMINOUS_HOME:-$HOME/.numinous}"

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

usage() {
    say "Numinous installer (macOS and Linux)."
    say ""
    say "  install.sh                  install or update Numinous"
    say "  install.sh --uninstall      remove ~/.numinous and the PATH lines it added"
    say "  install.sh --no-modify-path install without editing any shell profile"
    say ""
    say "NUMINOUS_HOME overrides the install root (default ~/.numinous)."
    say "Play history in ~/.numinous-journey and friends is never touched."
}

UNINSTALL=0
MODIFY_PATH=1
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall) UNINSTALL=1 ;;
        --no-modify-path) MODIFY_PATH=0 ;;
        -h | --help)
            usage
            exit 0
            ;;
        *) fail "unknown option '$1' (try --help)" ;;
    esac
    shift
done

case "$NUMINOUS_HOME" in
    "" | / | "$HOME") fail "NUMINOUS_HOME must name a dedicated absolute directory" ;;
    /*) ;;
    *) fail "NUMINOUS_HOME must be an absolute path" ;;
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
    && [ "$NUMINOUS_HOME" != "$DEFAULT_HOME" ] \
    && [ ! -f "$INSTALL_MARKER" ] \
    && ! directory_is_empty "$NUMINOUS_HOME" \
    && { [ ! -x "$BINARY_PATH/numinous" ] || [ ! -f "$SOURCE_PATH/Cargo.toml" ]; }; then
    fail "custom NUMINOUS_HOME exists but is not a recognized Numinous install root"
fi

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
# never a user's own PATH edits. The path marker only keeps re-runs from
# appending a duplicate when any line already provides the directory.
installer_note='added by the Numinous installer'
if [ "$NUMINOUS_HOME" = "$DEFAULT_HOME" ]; then
    path_line="export PATH=\"\$HOME/.numinous/bin:\$PATH\" # $installer_note"
    path_marker='.numinous/bin'
else
    quoted_bin_dir="$(posix_quote "$BINARY_PATH")"
    path_line="export PATH=$quoted_bin_dir:\$PATH # $installer_note"
    path_marker="$BINARY_PATH"
fi

strip_path_line() {
    profile="$1"
    [ -f "$profile" ] || return 0
    grep -Fq "$installer_note" "$profile" || return 0
    tmp="$profile.numinous-uninstall"
    grep -Fv "$installer_note" "$profile" >"$tmp" || true
    mv "$tmp" "$profile"
}

if [ "$UNINSTALL" -eq 1 ]; then
    cd "$install_parent"
    if [ -L "$install_name" ]; then
        fail "refusing to remove a symbolic-link install root: $NUMINOUS_HOME"
    fi
    if [ -e "$install_name" ] \
        && [ "$NUMINOUS_HOME" != "$DEFAULT_HOME" ] \
        && [ ! -f "$install_name/.numinous-install-root" ]; then
        fail "refusing to remove an unmarked custom install root: $NUMINOUS_HOME"
    fi
    rm -rf -- "$install_name"
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
printf '%s\n' 'Numinous install root' >.numinous-install-root
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

# The audio and window builds need the ALSA and xkbcommon headers on Linux
# (the same packages CI installs).
if [ "$os" = linux ]; then
    if ! have pkg-config || ! pkg-config --exists alsa xkbcommon 2>/dev/null; then
        fail "the build needs pkg-config plus the ALSA and xkbcommon headers. Install them, then re-run.
  Debian/Ubuntu: sudo apt-get install -y pkg-config libasound2-dev libxkbcommon-dev
  Fedora:        sudo dnf install -y pkgconf-pkg-config alsa-lib-devel libxkbcommon-devel
  Arch:          sudo pacman -S --needed pkgconf alsa-lib libxkbcommon
  openSUSE:      sudo zypper install pkg-config alsa-devel libxkbcommon-devel"
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
    if [ -z "$cargo_minor" ] || [ "$cargo_minor" -lt 85 ]; then
        fail "this cargo is older than the minimum supported Rust (1.85) and rustup is absent.
Install rustup from https://rustup.rs and re-run this installer"
    fi
    say "note: using the system cargo without rustup; the pinned toolchain file is ignored."
fi

# Fetch the source. git gives cheap updates; without it, download a snapshot.
# Either way the previous build cache is kept so updates do not start over.
if [ -d "$SRC_DIR/.git" ] && have git; then
    say "Updating the source in $SRC_DIR"
    git -C "$SRC_DIR" fetch --depth 1 origin main
    git -C "$SRC_DIR" reset --hard --quiet origin/main
else
    stage=".staging-$$"
    trap 'rm -rf "$stage"' EXIT
    rm -rf "$stage"
    mkdir -p "$stage"
    if have git; then
        say "Cloning $REPO_URL into $SRC_DIR"
        git clone --depth 1 "$REPO_URL" "$stage/src"
        new_tree="$stage/src"
    else
        have tar || fail "neither git nor tar is installed; install git and re-run"
        say "git is not installed; downloading a source snapshot instead."
        fetch "$SNAPSHOT_URL" "$stage/numinous.tar.gz"
        tar -xzf "$stage/numinous.tar.gz" -C "$stage"
        new_tree="$stage/numinous-main"
        [ -d "$new_tree" ] || fail "unexpected source snapshot layout"
    fi
    if [ -d "$SRC_DIR/target" ]; then
        mv "$SRC_DIR/target" "$new_tree/target"
    fi
    rm -rf "$SRC_DIR"
    mv "$new_tree" "$SRC_DIR"
    rm -rf "$stage"
    trap - EXIT
fi

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
    install -m 755 "$SRC_DIR/target/release/$binary" "$BIN_DIR/$binary"
done
# The app finds the built-in radio next to its executable.
ln -sfn "$SOURCE_PATH/assets/radio" "$BIN_DIR/radio"

if [ "$MODIFY_PATH" -eq 1 ]; then
    add_path_line() {
        profile="$1"
        if [ -f "$profile" ] && grep -Fq "$path_marker" "$profile"; then
            return 0
        fi
        printf '\n%s\n' "$path_line" >>"$profile"
    }
    add_path_line "$HOME/.profile"
    # A login bash reads .bash_profile instead of .profile when it exists.
    for profile in "$HOME/.bash_profile" "$HOME/.bashrc"; do
        if [ -f "$profile" ]; then
            add_path_line "$profile"
        fi
    done
    if [ -f "$HOME/.zshrc" ] || [ "${SHELL##*/}" = "zsh" ]; then
        add_path_line "$HOME/.zshrc"
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
    say "Open a new terminal so PATH picks up $BINARY_PATH, then type: numinous-app"
else
    say "PATH was not modified. Add this yourself, or run the binaries by full path:"
    say "  $path_line"
fi
say ""
say "Read PLAY.md first if you read anything: $SOURCE_PATH/PLAY.md"
say "Update any time by re-running this installer. Uninstall with --uninstall."
