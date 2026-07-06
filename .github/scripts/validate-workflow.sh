#!/bin/bash
# Validate GitHub Actions workflow configuration and prerequisites
# Usage: validate-workflow.sh

set -e

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[⚠]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

ERRORS=0
WARNINGS=0

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    log_error "Not a git repository"
    exit 1
fi

log_info "Starting workflow validation..."
echo ""

# 1. Check workflow file exists
log_info "Checking workflow file..."
if [[ -f ".github/workflows/upstream-release.yml" ]]; then
    log_success "Workflow file exists"
else
    log_error "Workflow file not found: .github/workflows/upstream-release.yml"
    ((ERRORS++))
fi

# 2. Check YAML syntax
log_info "Validating YAML syntax..."
if command -v yamllint &> /dev/null; then
    if yamllint -d relaxed .github/workflows/upstream-release.yml &> /dev/null; then
        log_success "YAML syntax valid"
    else
        log_warn "YAML syntax check failed (install yamllint for validation)"
        ((WARNINGS++))
    fi
else
    log_warn "yamllint not installed (skipping YAML validation)"
    ((WARNINGS++))
fi

# 3. Check Cargo.toml structure
log_info "Validating Cargo.toml files..."
REQUIRED_CRATES=("core" "ai3-lib" "mining" "extensions")
for crate in "${REQUIRED_CRATES[@]}"; do
    if [[ -f "${crate}/Cargo.toml" ]]; then
        VERSION=$(grep "^version" "${crate}/Cargo.toml" | head -1 | sed 's/version = "\(.*\)".*/\1/')
        if [[ -n "$VERSION" ]]; then
            log_success "${crate}: v${VERSION}"
        else
            log_error "${crate}/Cargo.toml missing version field"
            ((ERRORS++))
        fi
    else
        log_error "${crate}/Cargo.toml not found"
        ((ERRORS++))
    fi
done

# 4. Check git configuration
log_info "Checking git configuration..."
MAIN_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$MAIN_BRANCH" == "main" || "$MAIN_BRANCH" == "master" ]]; then
    log_success "Default branch appears to be main/master"
else
    log_warn "Not on main/master branch (currently on $MAIN_BRANCH)"
    ((WARNINGS++))
fi

# 5. Check recent commits
log_info "Checking commit history..."
if git log --oneline -5 &> /dev/null; then
    log_success "Commit history accessible"
else
    log_error "Cannot access commit history"
    ((ERRORS++))
fi

# 6. Check for upstream/downstream repository references
log_info "Checking downstream repository references..."
DOWNSTREAM_REPOS=(
    "TribeWarez/pot-o-core"
    "TribeWarez/ai3-lib"
    "TribeWarez/pot-o-mining"
    "TribeWarez/pot-o-extensions"
)

for repo in "${DOWNSTREAM_REPOS[@]}"; do
    if grep -q "$repo" .github/workflows/upstream-release.yml; then
        log_success "Found reference to $repo in workflow"
    else
        log_warn "No reference to $repo in workflow"
        ((WARNINGS++))
    fi
done

# 7. Check for helper scripts
log_info "Checking helper scripts..."
if [[ -f ".github/scripts/update-dependencies.sh" ]]; then
    log_success "Helper script found: .github/scripts/update-dependencies.sh"
    if [[ -x ".github/scripts/update-dependencies.sh" ]]; then
        log_success "Helper script is executable"
    else
        log_warn "Helper script is not executable (chmod +x recommended)"
        ((WARNINGS++))
    fi
else
    log_warn "Helper script not found: .github/scripts/update-dependencies.sh"
    ((WARNINGS++))
fi

# 8. Check documentation
log_info "Checking documentation..."
if [[ -f ".github/UPSTREAM_RELEASE.md" ]]; then
    log_success "Documentation found: .github/UPSTREAM_RELEASE.md"
else
    log_warn "Documentation not found: .github/UPSTREAM_RELEASE.md"
    ((WARNINGS++))
fi

# 9. Check Rust installation
log_info "Checking Rust toolchain..."
if command -v cargo &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    log_success "Rust installed: $RUST_VERSION"
else
    log_warn "Rust not found in PATH (required for local testing)"
    ((WARNINGS++))
fi

# 10. Check GitHub CLI (optional)
log_info "Checking GitHub CLI (optional)..."
if command -v gh &> /dev/null; then
    log_success "GitHub CLI installed"
else
    log_warn "GitHub CLI not found (optional for manual workflow management)"
    ((WARNINGS++))
fi

echo ""
echo "=========================================="
echo "Validation Summary:"
echo "=========================================="
echo -e "Errors:   ${RED}${ERRORS}${NC}"
echo -e "Warnings: ${YELLOW}${WARNINGS}${NC}"
echo "=========================================="

if [[ $ERRORS -eq 0 ]]; then
    if [[ $WARNINGS -eq 0 ]]; then
        log_success "All checks passed!"
        echo ""
        echo "Next steps:"
        echo "1. Push the .github directory to trigger workflow:"
        echo "   git add .github && git commit -m 'feat: add upstream release workflow'"
        echo ""
        echo "2. Create a release tag:"
        echo "   git tag pot-o-validator-v0.1.0"
        echo "   git push origin pot-o-validator-v0.1.0"
        echo ""
        echo "3. Monitor workflow execution:"
        echo "   https://github.com/TribeWarez/pot-o-validator/actions"
        exit 0
    else
        log_warn "Validation passed with $WARNINGS warning(s)"
        exit 0
    fi
else
    log_error "Validation failed with $ERRORS error(s)"
    exit 1
fi

