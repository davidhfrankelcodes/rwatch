#!/usr/bin/env bash
# Run rwatch's full test suite on all configured remote platforms.
#
# Configuration (set via env or scripts/.env.test — never committed):
#   RWATCH_SSH_MACOS   SSH connection string for the macOS remote, e.g. "user@host"
#   RWATCH_SSH_LINUX   SSH connection string for the Linux remote, e.g. "user@host"
#   RWATCH_CARGO_MACOS Path to cargo on the macOS remote (default: /opt/homebrew/bin/cargo)
#   RWATCH_CARGO_LINUX Path to cargo on the Linux remote (default: cargo)
#
# Usage:
#   bash scripts/test_all_platforms.sh
#   # or with inline env:
#   RWATCH_SSH_MACOS="user@host" RWATCH_SSH_LINUX="user@host" bash scripts/test_all_platforms.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load local config if present (not committed to git)
if [[ -f "$SCRIPT_DIR/.env.test" ]]; then
    # shellcheck disable=SC1091
    source "$SCRIPT_DIR/.env.test"
fi

SSH_MACOS="${RWATCH_SSH_MACOS:-}"
SSH_LINUX="${RWATCH_SSH_LINUX:-}"
CARGO_MACOS="${RWATCH_CARGO_MACOS:-/opt/homebrew/bin/cargo}"
CARGO_LINUX="${RWATCH_CARGO_LINUX:-cargo}"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
PASS=0
FAIL=0

run_remote() {
    local label="$1"
    local ssh_target="$2"
    local cargo_bin="$3"

    echo ""
    echo "════════════════════════════════════════"
    echo "  Testing on $label  (branch: $BRANCH)"
    echo "════════════════════════════════════════"

    if [[ -z "$ssh_target" ]]; then
        echo "  SKIPPED — no SSH target configured"
        echo "  Set RWATCH_SSH_$(echo "$label" | tr '[:lower:]' '[:upper:]' | tr ' ' '_') to enable"
        return
    fi

    if ssh "$ssh_target" "
        set -e
        cd ~/rwatch
        git fetch --quiet
        git checkout --quiet '$BRANCH'
        git pull --quiet
        export PATH=\"\$(dirname '$cargo_bin'):\$PATH\"
        '$cargo_bin' test 2>&1
    "; then
        echo "  ✓ PASSED on $label"
        PASS=$((PASS + 1))
    else
        echo "  ✗ FAILED on $label"
        FAIL=$((FAIL + 1))
    fi
}

run_remote "macOS" "$SSH_MACOS" "$CARGO_MACOS"
run_remote "Linux" "$SSH_LINUX" "$CARGO_LINUX"

echo ""
echo "════════════════════════════════════════"
echo "  Remote results: $PASS passed, $FAIL failed"
echo "════════════════════════════════════════"

[[ $FAIL -eq 0 ]]
