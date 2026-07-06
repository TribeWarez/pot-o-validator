# Upstream Release Workflow Documentation

## Overview

The **Upstream Release Workflow** is an automated GitHub Actions workflow that orchestrates the release and synchronization of the `pot-o-validator` monorepo and its downstream repositories. When a new release tag is pushed to `pot-o-validator`, the workflow automatically updates all dependent crates in their respective downstream repositories.

## Workflow Trigger

The workflow is triggered when a tag matching the pattern `pot-o-validator-v*` is pushed:

```bash
git tag pot-o-validator-v0.1.1
git push origin pot-o-validator-v0.1.1
```

## Architecture

The workflow is organized into 6 sequential stages:

### Stage 1: Validate
- **Job:** `validate`
- **Purpose:** Validate the release and extract version information
- **Actions:**
  - Checkout main repository
  - Extract version from git tag
  - Validate `Cargo.lock`
  - Build and test the validator with `--release` flag
  - Extract version numbers for all workspace crates
  - Output versions as job outputs for downstream jobs
- **Outputs:**
  - `version`: Validator release version
  - `core-version`: pot-o-core version
  - `ai3-version`: ai3-lib version
  - `mining-version`: pot-o-mining version
  - `extensions-version`: pot-o-extensions version

### Stage 2: Update pot-o-core
- **Job:** `update-pot-o-core`
- **Dependencies:** Requires `validate` to complete
- **Purpose:** Update pot-o-core repository with synchronized dependencies
- **Actions:**
  - Checkout pot-o-core repository using `GH_PAT` token
  - Configure git user (TribeWarez Release Bot)
  - Run `cargo update` to refresh dependency locks
  - Build and test release artifacts
  - Auto-commit changes with semantic message
  - Push to main branch
  - Create and push release tag

### Stage 3: Update ai3-lib
- **Job:** `update-ai3-lib`
- **Dependencies:** Requires `validate` to complete
- **Purpose:** Update ai3-lib with new pot-o-core dependency
- **Actions:**
  - Checkout ai3-lib repository
  - Update `pot-o-core` dependency to match workspace version
  - Run tests and validation
  - Commit and push changes
  - Create release tag

### Stage 4: Update pot-o-mining
- **Job:** `update-pot-o-mining`
- **Dependencies:** Requires `validate`, `update-pot-o-core`, and `update-ai3-lib` to complete
- **Purpose:** Update pot-o-mining with all upstream dependencies
- **Actions:**
  - Checkout pot-o-mining repository
  - Update `pot-o-core` and `ai3-lib` dependencies
  - Build and test with updated dependencies
  - Commit and push changes
  - Create release tag

### Stage 5: Update pot-o-extensions
- **Job:** `update-pot-o-extensions`
- **Dependencies:** Requires all previous stages to complete
- **Purpose:** Update pot-o-extensions with all upstream dependencies (final stage)
- **Actions:**
  - Checkout pot-o-extensions repository
  - Update all dependencies: `pot-o-core`, `ai3-lib`, `pot-o-mining`
  - Build and test with updated dependencies
  - Commit and push changes
  - Create release tag

### Stage 6: Verification and Notification
- **Job:** `verify-and-notify`
- **Dependencies:** Runs after all update jobs (regardless of outcome)
- **Purpose:** Create release notes and provide completion status
- **Actions:**
  - Generate release completion report
  - Create GitHub Release with detailed release notes (on success)
  - Report any failures

## Dependency Graph

```
validate
├── update-pot-o-core
├── update-ai3-lib
│   └── depends on: validate, update-pot-o-core
├── update-pot-o-mining
│   └── depends on: validate, update-pot-o-core, update-ai3-lib
├── update-pot-o-extensions
│   └── depends on: validate, update-pot-o-core, update-ai3-lib, update-pot-o-mining
└── verify-and-notify
    └── depends on: all previous jobs
```

This dependency graph ensures:
1. All updates are validated first
2. pot-o-core is updated independently (has minimal dependencies)
3. ai3-lib waits for pot-o-core to be updated
4. pot-o-mining waits for both pot-o-core and ai3-lib
5. pot-o-extensions waits for all three to be updated
6. Verification and GitHub Release creation happens last

## Prerequisites

### GitHub Secrets

The workflow requires a `GH_PAT` (Personal Access Token) secret configured in the repository:

1. Create a PAT with the following permissions:
   - `repo` (full control of private repositories)
   - `workflow` (update GitHub Actions workflows)
   - `write:packages` (write packages to GitHub Packages)

2. Add the secret to your GitHub repository:
   - Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `GH_PAT`
   - Value: Your personal access token

### Repository Configuration

Ensure all downstream repositories exist and are accessible:
- `TribeWarez/pot-o-core`
- `TribeWarez/ai3-lib`
- `TribeWarez/pot-o-mining`
- `TribeWarez/pot-o-extensions`

All repositories must have `main` branch as the default branch.

## Commit Messages

The workflow uses semantic commit messages for all auto-commits:

```
chore(deps): [dependency updates], sync with pot-o-validator upstream v[VERSION]
```

Examples:
- `chore(deps): sync with pot-o-validator upstream v0.1.1`
- `chore(deps): update pot-o-core to 0.1.0, sync with pot-o-validator upstream v0.1.1`
- `chore(deps): update pot-o-core to 0.1.0, ai3-lib to 0.1.0, pot-o-mining to 0.1.0, sync with pot-o-validator upstream v0.1.1`

## Release Versioning

### Version Extraction

Versions are extracted from `Cargo.toml` files in the monorepo:
- Validator version: From tag name (pot-o-validator-v**X.Y.Z**)
- Crate versions: From respective `Cargo.toml` `version` fields

### Semantic Versioning

All crates follow [Semantic Versioning](https://semver.org/):
- **MAJOR.MINOR.PATCH**
- MAJOR: Breaking API changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes

## Release Notes

When all stages complete successfully, a GitHub Release is created with:
- Release title: Validator version
- Release body: Contains
  - Updated versions for all downstream crates
  - Timeline of completed stages
  - Links to respective repository releases

## Monitoring and Troubleshooting

### Viewing Workflow Runs

1. Go to your repository on GitHub
2. Click the "Actions" tab
3. Select "Upstream Release - Update Downstream Repos"
4. Click on a workflow run to view detailed logs

### Common Issues

#### Issue: Workflow fails at validation stage
- Check: `Cargo.lock` is up-to-date
- Check: All workspace crates build successfully with `cargo build --release`
- Check: Tests pass with `cargo test --release`

#### Issue: Update job fails with authentication error
- Check: `GH_PAT` secret is configured correctly
- Check: Token has required permissions (`repo`, `workflow`)
- Check: Token is not expired
- Check: Downstream repository exists and is accessible

#### Issue: Dependency update fails
- Check: Cargo.toml syntax is valid in downstream repos
- Check: `cargo update` runs successfully locally
- Check: Dependency versions are compatible

#### Issue: Git push fails
- Check: Repository has `main` branch
- Check: Bot user has write permissions
- Check: No branch protection rules blocking the push

### Re-running Failed Workflows

If a workflow fails and you've fixed the issue:

1. Delete the tag locally and remotely:
   ```bash
   git tag -d pot-o-validator-v0.1.1
   git push origin :refs/tags/pot-o-validator-v0.1.1
   ```

2. Push the tag again:
   ```bash
   git tag pot-o-validator-v0.1.1
   git push origin pot-o-validator-v0.1.1
   ```

Or use GitHub's "Re-run failed jobs" option if available.

## Manual Release Process (Alternative)

If automated workflow fails, you can manually update downstream repos:

```bash
# For each downstream repo
cd /path/to/downstream-repo

# Update dependencies
sed -i 's/pot-o-core = ".*"/pot-o-core = "0.1.0"/' Cargo.toml
sed -i 's/ai3-lib = ".*"/ai3-lib = "0.1.0"/' Cargo.toml

# Update locks and test
cargo update
cargo build --release
cargo test --release

# Commit and push
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): update dependencies to match upstream release"
git push origin main

# Tag release
git tag pot-o-core-v0.1.0
git push origin pot-o-core-v0.1.0
```

## Environment Variables

The workflow sets the following environment variables for all jobs:

```yaml
RUST_BACKTRACE: 1
CARGO_TERM_COLOR: always
```

These enable:
- Full Rust backtraces for better error diagnostics
- Colored terminal output for improved readability

## Performance Considerations

### Caching

The workflow uses `Swatinem/rust-cache` to cache:
- Compiled artifacts
- Dependencies
- Build artifacts

This significantly reduces build times for subsequent runs.

### Parallel Execution

While the workflow has dependencies between stages, within each stage all independent tests and builds run in parallel. Stages run sequentially as they depend on previous completions.

## Security Considerations

### Token Permissions

The `GH_PAT` should be:
- Created with minimal required permissions
- Rotated regularly
- Never committed to the repository
- Stored only as a GitHub Secret

### Commit Verification

Consider enabling signed commits:
1. Configure GPG key in the bot account
2. Update workflow to sign commits
3. Add `--gpg-sign` flag to git commit command

## Future Enhancements

Potential improvements to the workflow:

1. **Crates.io Publishing** - Automatically publish updated crates to crates.io
2. **Documentation Publishing** - Update docs.rs with new documentation
3. **Slack Notifications** - Send detailed release notifications to Slack
4. **Changelog Management** - Automatically update CHANGELOG.md files
5. **Semantic Versioning** - Auto-bump versions based on conventional commits
6. **Pull Requests** - Create PRs instead of direct commits for review
7. **Container Registry** - Push Docker images to registry
8. **Release Artifacts** - Attach compiled binaries to GitHub Release

## Support

For issues or questions about the workflow:
1. Check this documentation
2. Review workflow logs in GitHub Actions
3. Consult the main repository's CONTRIBUTING.md
4. Create an issue in the repository

