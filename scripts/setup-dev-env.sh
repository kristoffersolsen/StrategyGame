#!/usr/bin/env bash
#
# setup-dev-env.sh — one-time developer environment setup.
#
# Run this once after cloning the repository:
#   ./scripts/setup-dev-env.sh
#
# What it does:
#   1. Verifies rustup is installed
#   2. Installs system libraries required by Bevy (Linux/apt only)
#   3. Installs the pinned Rust toolchain (1.95.0)
#   4. Installs required components (clippy, rustfmt)
#   5. Installs cargo-audit for dependency vulnerability scanning
#   6. Configures git to use the project's hook scripts
#   7. Verifies the workspace builds cleanly
#
# Safe to re-run — all steps are idempotent.

set -euo pipefail

# ------------- Helpers -------------

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # no color

ok()   { echo -e "  ${GREEN}✓${NC} $1"; }
fail() { echo -e "  ${RED}✗${NC} $1"; exit 1; }
info() { echo -e "  ${YELLOW}→${NC} $1"; }

echo ""
echo "Strategy Game — developer environment setup"
echo "============================================"
echo ""

# ------------- 1. Check rustup -------------

info "Checking for rustup..."
if ! command -v rustup &>/dev/null; then
    fail "rustup not found. Install it from https://rustup.rs and re-run this script."
fi
ok "rustup found ($(rustup --version 2>&1 | head -n1))"

# ------------- 2. System libraries (Linux/apt only) -------------
#
# Bevy requires several system libraries on Linux. On other platforms
# (macOS, Windows) these are handled differently and this step is skipped.

if [[ "$(uname -s)" == "Linux" ]]; then
    if command -v apt-get &>/dev/null; then
        BEVY_DEPS=(
            libasound2-dev    # audio (ALSA)
            libudev-dev       # input device support
            libwayland-dev    # Wayland display server
            libxkbcommon-dev  # keyboard handling
            pkg-config        # used by Bevy build scripts to locate libraries
        )

        # Check which packages are missing before calling sudo.
        MISSING=()
        for pkg in "${BEVY_DEPS[@]}"; do
            if ! dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q "install ok installed"; then
                MISSING+=("$pkg")
            fi
        done

        if [ ${#MISSING[@]} -eq 0 ]; then
            ok "Bevy system dependencies already installed"
        else
            info "Installing Bevy system dependencies (requires sudo)..."
            info "Packages: ${MISSING[*]}"
            sudo apt-get install -y "${MISSING[@]}"
            ok "Bevy system dependencies installed"
        fi
    else
        info "Non-apt Linux detected — skipping automatic system dependency install."
        info "Install the Bevy Linux dependencies for your distro manually if the build fails."
        info "Required: ALSA, udev, Wayland, xkbcommon, pkg-config."
        info "See: https://bevyengine.org/learn/quick-start/getting-started/#installing-os-dependencies"
    fi
elif [[ "$(uname -s)" == "Darwin" ]]; then
    info "macOS detected — skipping system dependency install (not required on macOS)."
else
    info "Unrecognised OS — skipping system dependency install. See Bevy docs if the build fails."
fi

# ------------- 3. Install pinned toolchain -------------

TOOLCHAIN="1.95.0"
info "Installing Rust toolchain $TOOLCHAIN..."
rustup toolchain install "$TOOLCHAIN" --profile minimal --no-self-update
ok "Rust $TOOLCHAIN installed"

# ------------- 4. Install required components -------------

info "Installing clippy and rustfmt..."
rustup component add clippy rustfmt --toolchain "$TOOLCHAIN"
ok "clippy and rustfmt installed"

# ------------- 5. Install cargo-audit -------------

info "Installing cargo-audit..."
if cargo audit --version &>/dev/null 2>&1; then
    ok "cargo-audit already installed ($(cargo audit --version))"
else
    cargo install cargo-audit --locked
    ok "cargo-audit installed"
fi

# ------------- 6. Configure git hooks -------------

info "Configuring git hooks..."
if [ ! -d ".git" ]; then
    fail "Not in a git repository root. Run this script from the repo root."
fi

git config core.hooksPath .githooks

# Ensure hook scripts are executable.
chmod +x .githooks/commit-msg
chmod +x .githooks/pre-commit

ok "Git hooks configured (.githooks/)"

# ------------- 7. Verify build -------------

info "Verifying workspace builds (this may take a few minutes on first run)..."
if cargo build --workspace --quiet; then
    ok "Workspace builds cleanly"
else
    fail "Build failed. Check the output above and fix before proceeding."
fi

# ------------- Done -------------

echo ""
echo "  Setup complete."
echo ""
echo "  Next steps:"
echo "    • Read CONTRIBUTING.md for the development workflow"
echo "    • Read STYLE_GUIDE.md for coding conventions"
echo "    • Read DOCUMENTATION_GUIDE.md for documentation conventions"
echo "    • Run 'cargo test --workspace' to verify the test suite passes"
echo ""
