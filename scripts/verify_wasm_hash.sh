#!/usr/bin/env bash
# verify_wasm_hash.sh — Verify that the on-chain WASM hash for a deployed
# YieldVault contract matches the locally-built (and optionally optimised)
# artefact.
#
# Usage:
#   ./scripts/verify_wasm_hash.sh <CONTRACT_ID> [OPTIONS]
#
# Options:
#   --network  <network>   Stellar network alias (default: testnet)
#   --wasm     <path>      Path to the local WASM file
#                          (default: target/wasm32-unknown-unknown/release/
#                                    yieldvault_contract.wasm)
#   --optimized            Use the optimised .optimized.wasm artefact instead
#   -h, --help             Show this help message and exit
#
# Exit codes:
#   0  Hashes match  — the on-chain contract matches the local build.
#   1  Hashes differ — the on-chain contract does NOT match the local build.
#   2  Pre-condition failure (missing argument, file, or dependency).
#
# Requirements:
#   stellar CLI  https://github.com/stellar/stellar-cli
#   sha256sum    (coreutils) or shasum (macOS)
#
# Example:
#   # Build first, then verify against testnet
#   make build
#   ./scripts/verify_wasm_hash.sh CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4
#
#   # Verify an optimised build against mainnet
#   make optimize
#   ./scripts/verify_wasm_hash.sh CA... --network mainnet --optimized

set -euo pipefail

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { echo -e "${BOLD}[INFO]${RESET}  $*"; }
success() { echo -e "${GREEN}[OK]${RESET}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${RESET}  $*"; }
error()   { echo -e "${RED}[ERROR]${RESET} $*" >&2; }

usage() {
    sed -n '/^# Usage:/,/^[^#]/{ /^#/{ s/^# \{0,1\}//; p } }' "$0"
    exit 2
}

# Compute SHA-256 of a file, returning only the hex digest.
sha256_file() {
    local file="$1"
    if command -v sha256sum &>/dev/null; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        error "Neither 'sha256sum' nor 'shasum' found. Install coreutils."
        exit 2
    fi
}

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

CONTRACT_ID=""
NETWORK="testnet"
WASM_PATH=""
USE_OPTIMIZED=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)   usage ;;
        --network)   NETWORK="$2";   shift 2 ;;
        --wasm)      WASM_PATH="$2"; shift 2 ;;
        --optimized) USE_OPTIMIZED=true; shift ;;
        -*)
            error "Unknown option: $1"
            usage
            ;;
        *)
            if [[ -z "$CONTRACT_ID" ]]; then
                CONTRACT_ID="$1"
                shift
            else
                error "Unexpected argument: $1"
                usage
            fi
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Pre-condition checks
# ---------------------------------------------------------------------------

if [[ -z "$CONTRACT_ID" ]]; then
    error "CONTRACT_ID is required."
    usage
fi

if ! command -v stellar &>/dev/null; then
    error "'stellar' CLI not found. Install it from https://github.com/stellar/stellar-cli"
    exit 2
fi

# Resolve WASM path
if [[ -z "$WASM_PATH" ]]; then
    BASE_WASM="target/wasm32-unknown-unknown/release/yieldvault_contract.wasm"
    if $USE_OPTIMIZED; then
        WASM_PATH="${BASE_WASM%.wasm}.optimized.wasm"
    else
        WASM_PATH="$BASE_WASM"
    fi
fi

if [[ ! -f "$WASM_PATH" ]]; then
    error "Local WASM artefact not found: $WASM_PATH"
    error "Run 'make build'$(if $USE_OPTIMIZED; then echo " && make optimize"; fi) first."
    exit 2
fi

# ---------------------------------------------------------------------------
# Compute local hash
# ---------------------------------------------------------------------------

info "Local artefact : $WASM_PATH"
LOCAL_HASH=$(sha256_file "$WASM_PATH")
info "Local SHA-256  : $LOCAL_HASH"

# ---------------------------------------------------------------------------
# Fetch on-chain hash via stellar CLI
#
# `stellar contract info --contract-id <ID> --network <net>` returns JSON that
# includes a "wasm_hash" field containing the hex-encoded SHA-256 of the
# uploaded WASM blob.  We extract it with a simple grep/awk so the script has
# no dependency on jq.
# ---------------------------------------------------------------------------

info "Fetching on-chain WASM hash for contract $CONTRACT_ID on $NETWORK …"

CONTRACT_INFO=$(stellar contract info \
    --contract-id "$CONTRACT_ID" \
    --network "$NETWORK" 2>&1) || {
    error "Failed to retrieve contract info."
    error "Output: $CONTRACT_INFO"
    exit 2
}

# The stellar CLI prints a table or JSON; extract the wasm_hash value.
# Try JSON key first ("wasm_hash": "..."), then table/plain format.
ONCHAIN_HASH=$(echo "$CONTRACT_INFO" \
    | grep -oE '"wasm_hash"\s*:\s*"[0-9a-fA-F]+"' \
    | grep -oE '[0-9a-fA-F]{64}' \
    | head -n1)

# Fallback: plain hex on a line labelled "wasm_hash"
if [[ -z "$ONCHAIN_HASH" ]]; then
    ONCHAIN_HASH=$(echo "$CONTRACT_INFO" \
        | grep -iE 'wasm.?hash' \
        | grep -oE '[0-9a-fA-F]{64}' \
        | head -n1)
fi

if [[ -z "$ONCHAIN_HASH" ]]; then
    error "Could not parse the on-chain WASM hash from stellar CLI output."
    error "Raw output:"
    echo "$CONTRACT_INFO" >&2
    exit 2
fi

info "On-chain SHA-256: $ONCHAIN_HASH"

# ---------------------------------------------------------------------------
# Compare
# ---------------------------------------------------------------------------

# Normalise to lowercase for a case-insensitive comparison.
LOCAL_LOWER=$(echo "$LOCAL_HASH"   | tr '[:upper:]' '[:lower:]')
ONCHAIN_LOWER=$(echo "$ONCHAIN_HASH" | tr '[:upper:]' '[:lower:]')

echo ""
if [[ "$LOCAL_LOWER" == "$ONCHAIN_LOWER" ]]; then
    success "WASM hash MATCHES — the on-chain contract is identical to the local build."
    echo ""
    echo "  Contract : $CONTRACT_ID"
    echo "  Network  : $NETWORK"
    echo "  Hash     : $LOCAL_HASH"
    exit 0
else
    error "WASM hash MISMATCH — the on-chain contract does NOT match the local build!"
    echo ""
    echo "  Contract  : $CONTRACT_ID"
    echo "  Network   : $NETWORK"
    echo "  Local     : $LOCAL_HASH"
    echo "  On-chain  : $ONCHAIN_HASH"
    echo ""
    warn "This may indicate the contract was deployed from a different build,"
    warn "or that the binary was modified after deployment. Investigate before use."
    exit 1
fi
