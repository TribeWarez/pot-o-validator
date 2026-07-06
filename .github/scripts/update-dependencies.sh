#!/bin/bash
# Update dependencies in a downstream repository's Cargo.toml
# Usage: update-dependencies.sh <crate-name> <version> [additional-crates...]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Validate arguments
if [[ $# -lt 2 ]]; then
    log_error "Usage: update-dependencies.sh <crate-name> <version> [crate-name] [version] ..."
    exit 1
fi

CARGO_TOML="./Cargo.toml"

if [[ ! -f "$CARGO_TOML" ]]; then
    log_error "Cargo.toml not found in current directory"
    exit 1
fi

log_info "Starting dependency updates..."

# Process pairs of crate-name and version
while [[ $# -ge 2 ]]; do
    CRATE_NAME="$1"
    VERSION="$2"
    shift 2

    log_info "Updating ${CRATE_NAME} to ${VERSION}"

    # Check if dependency exists in Cargo.toml
    if grep -q "^${CRATE_NAME} = " "$CARGO_TOML"; then
        # Update version in Cargo.toml
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS sed syntax
            sed -i '' "s/${CRATE_NAME} = \"[^\"]*\"/${CRATE_NAME} = \"${VERSION}\"/" "$CARGO_TOML"
        else
            # Linux sed syntax
            sed -i "s/${CRATE_NAME} = \"[^\"]*\"/${CRATE_NAME} = \"${VERSION}\"/" "$CARGO_TOML"
        fi
        log_info "Updated ${CRATE_NAME} to ${VERSION}"
    else
        log_warn "${CRATE_NAME} not found in dependencies, skipping"
    fi
done

# Run cargo update to resolve all dependencies
log_info "Running cargo update..."
cargo update

log_info "Dependency updates completed successfully"

