#!/usr/bin/env bash
# Run rwatch's full test suite on all configured remote platforms.
#
# For each platform this script:
#   0. Setup  — wipes the remote work dir and clones the current branch fresh
#   1. Phase 1 — cargo test          (debug build + full test suite)
#   2. Phase 2 — cargo build --release (produce the release binary)
#   3. Phase 3 — cargo test --release  (full test suite against the release binary)
#
# The repo URL is read from `git remote get-url origin` automatically.
# Make sure the current branch is pushed before running.
#
# Configuration (set via env or scripts/.env.test — never committed):
#   RWATCH_SSH_MACOS    SSH connection string for the macOS remote, e.g. "user@host"
#   RWATCH_SSH_LINUX    SSH connection string for the Linux remote, e.g. "user@host"
#   RWATCH_DIR_MACOS    Working directory on the macOS remote (default: ~/rwatch-ci)
#   RWATCH_DIR_LINUX    Working directory on the Linux remote (default: ~/rwatch-ci)
#   RWATCH_CARGO_MACOS  Path to cargo on the macOS remote (default: /opt/homebrew/bin/cargo)
#   RWATCH_CARGO_LINUX  Path to cargo on the Linux remote (default: cargo)
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
DIR_MACOS="${RWATCH_DIR_MACOS:-~/rwatch-ci}"
DIR_LINUX="${RWATCH_DIR_LINUX:-~/rwatch-ci}"
CARGO_MACOS="${RWATCH_CARGO_MACOS:-/opt/homebrew/bin/cargo}"
CARGO_LINUX="${RWATCH_CARGO_LINUX:-cargo}"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
# Normalize SSH remote URLs (git@github.com:user/repo.git) to HTTPS so the
# remote hosts can clone without needing GitHub in their known_hosts.
REPO_URL="$(git remote get-url origin | sed 's|git@github\.com:|https://github.com/|')"
PASS=0
FAIL=0

# run_phase LABEL SSH_TARGET PHASE_NUM PHASE_DESC REMOTE_CMD
# Prints a header, runs the SSH command, and returns its exit code.
run_phase() {
    local label="$1"
    local ssh_target="$2"
    local phase_num="$3"
    local phase_desc="$4"
    local remote_cmd="$5"

    echo ""
    echo "  ── Phase $phase_num: $phase_desc ──"
    if ssh "$ssh_target" "$remote_cmd"; then
        echo "  ✓ Phase $phase_num PASSED on $label"
        return 0
    else
        echo "  ✗ Phase $phase_num FAILED on $label"
        return 1
    fi
}

run_remote() {
    local label="$1"
    local ssh_target="$2"
    local cargo_bin="$3"
    local remote_dir="$4"
    local platform_fail=0

    echo ""
    echo "════════════════════════════════════════"
    echo "  Platform: $label  (branch: $BRANCH)"
    echo "════════════════════════════════════════"

    if [[ -z "$ssh_target" ]]; then
        echo "  SKIPPED — no SSH target configured"
        echo "  Set RWATCH_SSH_$(echo "$label" | tr '[:lower:]' '[:upper:]' | tr ' ' '_') to enable"
        return
    fi

    # Setup: wipe any previous run and clone the current branch fresh
    echo ""
    echo "  ── Setup: fresh clone into $remote_dir ──"
    if ssh "$ssh_target" "
        set -e
        mkdir -p '$remote_dir'
        rm -rf '$remote_dir'
        git clone --quiet --branch '$BRANCH' '$REPO_URL' '$remote_dir'
    "; then
        echo "  ✓ Setup PASSED on $label"
    else
        echo "  ✗ Setup FAILED on $label — skipping phases"
        FAIL=$((FAIL + 1))
        return
    fi

    # Phase 1: debug build + full test suite
    run_phase "$label" "$ssh_target" 1 "cargo test (debug)" "
        set -e
        cd '$remote_dir'
        export PATH=\"\$(dirname '$cargo_bin'):\$PATH\"
        '$cargo_bin' test 2>&1
    " || platform_fail=$((platform_fail + 1))

    # Phase 2: release build
    run_phase "$label" "$ssh_target" 2 "cargo build --release" "
        set -e
        cd '$remote_dir'
        export PATH=\"\$(dirname '$cargo_bin'):\$PATH\"
        '$cargo_bin' build --release 2>&1
    " || platform_fail=$((platform_fail + 1))

    # Phase 3: full test suite against the release binary
    run_phase "$label" "$ssh_target" 3 "cargo test --release" "
        set -e
        cd '$remote_dir'
        export PATH=\"\$(dirname '$cargo_bin'):\$PATH\"
        '$cargo_bin' test --release 2>&1
    " || platform_fail=$((platform_fail + 1))

    echo ""
    if [[ $platform_fail -eq 0 ]]; then
        echo "  ✓ All phases PASSED on $label"
        PASS=$((PASS + 1))
    else
        echo "  ✗ $platform_fail phase(s) FAILED on $label"
        FAIL=$((FAIL + 1))
    fi
}

run_remote "macOS" "$SSH_MACOS" "$CARGO_MACOS" "$DIR_MACOS"
run_remote "Linux" "$SSH_LINUX" "$CARGO_LINUX" "$DIR_LINUX"

echo ""
echo "════════════════════════════════════════"
echo "  Remote results: $PASS platform(s) fully passed, $FAIL failed"
echo "════════════════════════════════════════"

[[ $FAIL -eq 0 ]]
