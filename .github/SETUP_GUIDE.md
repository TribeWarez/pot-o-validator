# GitHub Actions Upstream Release Workflow - Setup Guide

This guide walks you through setting up and configuring the upstream release workflow for the `pot-o-validator` project.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Configuration Steps](#configuration-steps)
4. [Verifying Setup](#verifying-setup)
5. [Creating Your First Release](#creating-your-first-release)
6. [Troubleshooting](#troubleshooting)

## Overview

The upstream release workflow automatically updates all downstream repositories (pot-o-core, ai3-lib, pot-o-mining, pot-o-extensions) when a new release tag is created in the pot-o-validator repository.

**Trigger:** Pushing a tag matching `pot-o-validator-v*`

**Outcome:** Automatic updates to all downstream repositories with new dependency versions

## Prerequisites

Before setting up the workflow, ensure you have:

### 1. GitHub Account with Repository Access
- Administrator or maintainer access to `TribeWarez/pot-o-validator`
- Access to all downstream repositories:
  - `TribeWarez/pot-o-core`
  - `TribeWarez/ai3-lib`
  - `TribeWarez/pot-o-mining`
  - `TribeWarez/pot-o-extensions`

### 2. Personal Access Token (PAT)
Create a GitHub Personal Access Token with the following permissions:

1. Go to [GitHub Settings → Personal access tokens → Tokens (classic)](https://github.com/settings/tokens)
2. Click "Generate new token (classic)"
3. Configure the token:
   - **Token name:** `tribewarez-release-bot`
   - **Expiration:** 90 days or longer
   - **Scopes:**
     - ☑ repo (Full control of private repositories)
     - ☑ workflow (Update GitHub Actions workflows)
     - ☑ write:packages (Write packages to GitHub Packages)
4. Click "Generate token"
5. **Copy the token** (you won't see it again)

### 3. Local Tools
- `git` (for version control)
- `bash` (for running setup scripts)
- `cargo` (for validating Rust configuration)
- `gh` (GitHub CLI - optional but recommended)

## Configuration Steps

### Step 1: Clone the Repository

```bash
cd /path/to/projects
git clone https://github.com/TribeWarez/pot-o-validator.git
cd pot-o-validator
```

### Step 2: Add GitHub Secret

#### Option A: Using GitHub CLI (Recommended)

```bash
# Authenticate with GitHub
gh auth login

# Set the secret
gh secret set GH_PAT

# When prompted, paste your personal access token
```

#### Option B: Using GitHub Web Interface

1. Navigate to your repository on GitHub
2. Go to **Settings → Secrets and variables → Actions**
3. Click **New repository secret**
4. Configure the secret:
   - **Name:** `GH_PAT`
   - **Value:** [Paste your personal access token]
5. Click **Add secret**

#### Option C: Using the Setup Script

```bash
chmod +x .github/scripts/setup-workflow.sh
./.github/scripts/setup-workflow.sh
```

This interactive script will guide you through the configuration.

### Step 3: Verify Workflow Files

Ensure all workflow files are in place:

```bash
# Check workflow file
ls -la .github/workflows/upstream-release.yml

# Check helper scripts
ls -la .github/scripts/

# Check documentation
ls -la .github/UPSTREAM_RELEASE.md
```

Expected output:
```
.github/workflows/upstream-release.yml     (900+ lines)
.github/scripts/update-dependencies.sh     (executable)
.github/scripts/validate-workflow.sh       (executable)
.github/scripts/setup-workflow.sh          (executable)
.github/UPSTREAM_RELEASE.md                (documentation)
```

### Step 4: Make Scripts Executable

```bash
chmod +x .github/scripts/*.sh
```

### Step 5: Validate Configuration

```bash
./.github/scripts/validate-workflow.sh
```

This script checks:
- Workflow file syntax
- Cargo.toml structure
- Git configuration
- Helper scripts
- Rust toolchain
- GitHub CLI (optional)

## Verifying Setup

### Quick Verification

```bash
# Check that workflow file contains proper syntax
head -20 .github/workflows/upstream-release.yml

# Check for secret configuration
gh secret list
```

### Full Verification Using Setup Script

```bash
./.github/scripts/setup-workflow.sh
```

### Verify Downstream Repositories

Using GitHub CLI:

```bash
gh repo view TribeWarez/pot-o-core
gh repo view TribeWarez/ai3-lib
gh repo view TribeWarez/pot-o-mining
gh repo view TribeWarez/pot-o-extensions
```

## Creating Your First Release

### Step 1: Prepare Release

Ensure all code is committed:

```bash
git status
# Should show "nothing to commit, working tree clean"
```

### Step 2: Verify Cargo.toml Versions

```bash
# Check main validator version
grep '^version' Cargo.toml

# Check workspace crate versions
grep '^version' core/Cargo.toml
grep '^version' ai3-lib/Cargo.toml
grep '^version' mining/Cargo.toml
grep '^version' extensions/Cargo.toml
```

Expected output example:
```
pot-o-validator: 0.2.0
pot-o-core: 0.2.0
ai3-lib: 0.2.0
pot-o-mining: 0.2.0
pot-o-extensions: 0.2.0
```

### Step 3: Create Release Tag

```bash
# Create annotated tag
git tag -a pot-o-validator-v0.2.0 -m "Release v0.2.0"

# Verify tag was created
git tag -l

# Push tag to GitHub
git push origin pot-o-validator-v0.2.0
```

Or using GitHub CLI:

```bash
gh release create pot-o-validator-v0.2.0 --generate-notes
```

### Step 4: Monitor Workflow Execution

Using GitHub CLI:

```bash
# Watch workflow in real-time
gh run list --workflow=upstream-release.yml --limit=1

# View detailed logs
gh run view <RUN_ID> --log
```

Or using GitHub Web:

1. Go to your repository
2. Click **Actions**
3. Select **Upstream Release - Update Downstream Repos**
4. Click the most recent run

### Step 5: Verify Downstream Updates

Check that downstream repositories were updated:

```bash
gh repo view TribeWarez/pot-o-core
gh repo view TribeWarez/ai3-lib
gh repo view TribeWarez/pot-o-mining
gh repo view TribeWarez/pot-o-extensions
```

Each should have:
- Updated commits with dependency updates
- New release tags created
- Updated Cargo.lock files

## Troubleshooting

### Issue: Workflow doesn't trigger

**Symptom:** Tag is pushed but workflow doesn't run

**Solutions:**

1. **Check tag format:**
   ```bash
   git tag -l
   # Must match pattern: pot-o-validator-v*
   ```

2. **Verify tag was pushed:**
   ```bash
   git push origin pot-o-validator-v0.2.0
   ```

3. **Check workflow file syntax:**
   ```bash
   ./.github/scripts/validate-workflow.sh
   ```

### Issue: Workflow fails with authentication error

**Symptom:** Jobs fail with "403 Forbidden" or "Authentication failed"

**Solutions:**

1. **Verify secret is configured:**
   ```bash
   gh secret list
   # Should show GH_PAT
   ```

2. **Check token permissions:**
   - Go to https://github.com/settings/tokens
   - Verify token has `repo` and `workflow` scopes
   - Token should not be expired

3. **Recreate token if needed:**
   ```bash
   # Create new token with proper permissions
   # Then update secret:
   gh secret set GH_PAT
   ```

4. **Verify downstream repo access:**
   ```bash
   gh repo view TribeWarez/pot-o-core
   # Should succeed without errors
   ```

### Issue: Dependency update fails

**Symptom:** Workflow fails at cargo update or cargo build step

**Solutions:**

1. **Test locally:**
   ```bash
   cargo build --release
   cargo test --release
   ```

2. **Check Cargo.toml syntax in downstream repos:**
   ```bash
   # Clone downstream repo and verify
   git clone https://github.com/TribeWarez/pot-o-core.git
   cd pot-o-core
   cargo check
   ```

3. **Verify dependency versions are compatible:**
   ```bash
   cargo tree
   ```

### Issue: Git push fails

**Symptom:** Workflow fails with "permission denied" when pushing

**Solutions:**

1. **Verify branch exists:**
   ```bash
   gh repo view TribeWarez/pot-o-core --json defaultBranchRef
   # Should show "main" as default branch
   ```

2. **Check branch protection rules:**
   - Go to downstream repository
   - Settings → Branches
   - Verify `main` branch doesn't require status checks that would fail

3. **Verify token has write permissions:**
   - Token must have `repo` scope (full control)
   - Update token if needed

### Issue: Release notes not generated

**Symptom:** GitHub Release is created but without body/description

**Solutions:**

1. **Verify all stages completed successfully:**
   ```bash
   gh run view <RUN_ID>
   # All jobs should show "passed"
   ```

2. **Check release notes template:**
   - Workflow uses hardcoded template
   - Verify template section in `upstream-release.yml`

## Common Commands Reference

```bash
# List all release tags
git tag -l

# Create and push a release tag
git tag -a pot-o-validator-v0.2.0 -m "Release message"
git push origin pot-o-validator-v0.2.0

# Monitor workflow execution
gh run list --workflow=upstream-release.yml

# View workflow logs
gh run view <RUN_ID>

# Re-run failed workflow
gh run rerun <RUN_ID>

# View repository secrets
gh secret list

# Update a secret
gh secret set SECRET_NAME

# Check downstream repository status
gh repo view TribeWarez/pot-o-core
```

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Semantic Versioning](https://semver.org/)
- [GitHub CLI Documentation](https://cli.github.com/manual)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

## Support

For issues or questions:

1. Check the [Workflow Documentation](./UPSTREAM_RELEASE.md)
2. Review workflow logs in GitHub Actions
3. Run validation scripts locally
4. Create an issue in the repository

