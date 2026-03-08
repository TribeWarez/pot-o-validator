#!/bin/bash
# Setup script for configuring the upstream release workflow
# This script helps users set up the required GitHub secrets and validate configuration

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

print_header() {
    echo ""
    echo "=========================================="
    echo "$1"
    echo "=========================================="
}

print_header "Upstream Release Workflow Setup"

# Check if GitHub CLI is installed
if ! command -v gh &> /dev/null; then
    log_warn "GitHub CLI (gh) is not installed"
    echo "Installation instructions: https://cli.github.com/"
    echo ""
    read -p "Continue without GitHub CLI? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    GITHUB_CLI_AVAILABLE=false
else
    GITHUB_CLI_AVAILABLE=true
    log_success "GitHub CLI installed"
fi

echo ""
print_header "Prerequisites"

# Check if in git repository
if git rev-parse --git-dir > /dev/null 2>&1; then
    log_success "Git repository detected"
    REPO_URL=$(git config --get remote.origin.url)
    log_info "Repository: $REPO_URL"
else
    log_error "Not in a git repository"
    exit 1
fi

# Check if .github/workflows directory exists
if [[ -d ".github/workflows" ]]; then
    log_success ".github/workflows directory exists"
else
    mkdir -p .github/workflows
    log_success "Created .github/workflows directory"
fi

echo ""
print_header "Required Secrets Configuration"

echo "The workflow requires the following GitHub secret:"
echo ""
echo "Secret Name: GH_PAT"
echo "Description: Personal Access Token for downstream repository access"
echo ""
echo "Required Permissions:"
echo "  - repo (Full control of private repositories)"
echo "  - workflow (Update GitHub Actions workflows)"
echo "  - write:packages (Write packages to GitHub Packages)"
echo ""

if [[ "$GITHUB_CLI_AVAILABLE" == true ]]; then
    read -p "Would you like to create the secret using GitHub CLI? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        read -sp "Enter your GitHub Personal Access Token: " PAT
        echo

        # Extract owner/repo from URL
        if [[ $REPO_URL =~ git@github\.com:([^/]+)/([^/]+)(\.git)?$ ]]; then
            OWNER="${BASH_REMATCH[1]}"
            REPO="${BASH_REMATCH[2]}"
        elif [[ $REPO_URL =~ https://github\.com/([^/]+)/([^/]+)(\.git)?$ ]]; then
            OWNER="${BASH_REMATCH[1]}"
            REPO="${BASH_REMATCH[2]}"
        else
            log_error "Could not parse repository information from: $REPO_URL"
            exit 1
        fi

        log_info "Setting secret for $OWNER/$REPO..."
        echo "$PAT" | gh secret set GH_PAT --repo "$OWNER/$REPO"
        log_success "Secret GH_PAT created"
    fi
else
    echo "To create the secret manually:"
    echo "1. Go to your GitHub repository"
    echo "2. Settings → Secrets and variables → Actions"
    echo "3. Click 'New repository secret'"
    echo "4. Name: GH_PAT"
    echo "5. Value: Your GitHub Personal Access Token"
    echo "6. Click 'Add secret'"
fi

echo ""
print_header "Workflow File Status"

# Check if workflow file exists
if [[ -f ".github/workflows/upstream-release.yml" ]]; then
    log_success "Workflow file exists: .github/workflows/upstream-release.yml"
    WF_SIZE=$(wc -c < .github/workflows/upstream-release.yml)
    log_info "File size: $WF_SIZE bytes"
else
    log_warn "Workflow file not found"
fi

# Check helper scripts
if [[ -f ".github/scripts/update-dependencies.sh" ]]; then
    log_success "Helper script found: .github/scripts/update-dependencies.sh"
else
    log_warn "Helper script not found"
fi

if [[ -f ".github/UPSTREAM_RELEASE.md" ]]; then
    log_success "Documentation found: .github/UPSTREAM_RELEASE.md"
else
    log_warn "Documentation not found"
fi

echo ""
print_header "Downstream Repository Verification"

DOWNSTREAM_REPOS=(
    "TribeWarez/pot-o-core"
    "TribeWarez/ai3-lib"
    "TribeWarez/pot-o-mining"
    "TribeWarez/pot-o-extensions"
)

echo "Checking accessibility of downstream repositories..."
echo "(Note: This requires GitHub CLI and authentication)"
echo ""

if [[ "$GITHUB_CLI_AVAILABLE" == true ]]; then
    for repo in "${DOWNSTREAM_REPOS[@]}"; do
        if gh repo view "$repo" &> /dev/null; then
            log_success "Accessible: $repo"
        else
            log_warn "Could not verify: $repo (may not exist or not accessible)"
        fi
    done
else
    log_warn "GitHub CLI not available - skipping repository verification"
    echo "Please manually verify that these repositories exist and are accessible:"
    for repo in "${DOWNSTREAM_REPOS[@]}"; do
        echo "  - https://github.com/$repo"
    done
fi

echo ""
print_header "Testing Workflow Configuration"

# Run validation script if available
if [[ -x ".github/scripts/validate-workflow.sh" ]]; then
    log_info "Running workflow validation script..."
    .github/scripts/validate-workflow.sh
else
    log_warn "Validation script not found or not executable"
fi

echo ""
print_header "Next Steps"

echo "1. Commit and push the workflow files:"
echo "   git add .github"
echo "   git commit -m 'feat: add upstream release workflow'"
echo "   git push origin main"
echo ""

echo "2. Create and push a release tag to test the workflow:"
echo "   git tag pot-o-validator-v0.1.1"
echo "   git push origin pot-o-validator-v0.1.1"
echo ""

echo "3. Monitor the workflow execution:"
if [[ "$GITHUB_CLI_AVAILABLE" == true ]]; then
    echo "   gh run list --workflow=upstream-release.yml"
else
    echo "   https://github.com/TribeWarez/pot-o-validator/actions"
fi
echo ""

echo "4. For detailed logs:"
echo "   https://github.com/TribeWarez/pot-o-validator/actions/workflows/upstream-release.yml"
echo ""

echo "5. Read the full documentation:"
echo "   cat .github/UPSTREAM_RELEASE.md"
echo ""

log_success "Setup complete!"

