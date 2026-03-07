# Upstream Release Workflow - Quick Reference

## TL;DR - Quick Start

```bash
# 1. Set up secrets (one-time)
gh secret set TRIBEWAREZ_PAT  # Paste your GitHub PAT

# 2. Create a release
git tag pot-o-validator-v0.2.0
git push origin pot-o-validator-v0.2.0

# 3. Watch the workflow
gh run list --workflow=upstream-release.yml
```

## Workflow Diagram

```
┌─────────────┐
│  Git Tag    │
│ v0.2.0      │
└──────┬──────┘
       │
       ▼
┌──────────────────┐
│ Stage 1: Validate│ ← Extract versions, build, test
└────────┬─────────┘
         │
    ┌────┴─────┐
    │           │
    ▼           ▼
┌────────┐  ┌─────────┐
│pot-o-  │  │ai3-lib  │
│core    │  │         │
└────┬───┘  └────┬────┘
     │           │
     └───┬───────┘
         ▼
    ┌─────────────┐
    │pot-o-mining │
    └────┬────────┘
         │
         ▼
    ┌──────────────┐
    │pot-o-        │
    │extensions    │
    └────┬─────────┘
         │
         ▼
    ┌──────────────┐
    │Verify & Tag  │
    │Create Release│
    └──────────────┘
```

## Trigger Event

**When:** Push tag matching `pot-o-validator-v*`

```bash
git tag pot-o-validator-v0.2.0  # Triggers workflow
git push origin pot-o-validator-v0.2.0
```

**Pattern Matching:**
- ✅ pot-o-validator-v0.1.0
- ✅ pot-o-validator-v1.2.3
- ✅ pot-o-validator-v0.0.1-rc1
- ❌ pot-o-validator-0.1.0 (missing 'v')
- ❌ validator-v0.1.0 (wrong prefix)

## Required Configuration

### GitHub Secret: TRIBEWAREZ_PAT

Create at: Repository → Settings → Secrets and variables → Actions

**Value:** GitHub Personal Access Token with:
- `repo` (full control)
- `workflow` (update workflows)
- `write:packages` (write packages)

```bash
# Using GitHub CLI
gh secret set TRIBEWAREZ_PAT
# Then paste token when prompted
```

## Workflow Stages

| Stage | Job | Duration | Purpose |
|-------|-----|----------|---------|
| 1 | `validate` | ~2-3 min | Validate release, extract versions |
| 2 | `update-pot-o-core` | ~2-3 min | Update pot-o-core repo |
| 3 | `update-ai3-lib` | ~2-3 min | Update ai3-lib repo |
| 4 | `update-pot-o-mining` | ~2-3 min | Update pot-o-mining repo |
| 5 | `update-pot-o-extensions` | ~3-4 min | Update pot-o-extensions repo |
| 6 | `verify-and-notify` | ~1 min | Create GitHub Release |

**Total Duration:** ~15-20 minutes

## Monitoring Workflow

### GitHub CLI

```bash
# List recent runs
gh run list --workflow=upstream-release.yml

# Watch specific run
gh run watch <RUN_ID>

# View logs
gh run view <RUN_ID> --log

# Re-run on failure
gh run rerun <RUN_ID>
```

### GitHub Web

1. Go to: https://github.com/TribeWarez/pot-o-validator/actions
2. Select: "Upstream Release - Update Downstream Repos"
3. Click latest run
4. View logs for each job

## Version Extraction

Versions are automatically extracted from Cargo.toml files:

```bash
# Validator version (from tag)
pot-o-validator-v0.2.0  # → 0.2.0

# Workspace crate versions (from Cargo.toml)
core/Cargo.toml         # version = "0.2.0"
ai3-lib/Cargo.toml      # version = "0.2.0"
mining/Cargo.toml       # version = "0.2.0"
extensions/Cargo.toml   # version = "0.2.0"
```

## Commit Messages

Each downstream update creates commits:

```
chore(deps): update pot-o-core to 0.2.0, ai3-lib to 0.2.0, ...
```

Format: `chore(deps): [updates], sync with pot-o-validator upstream v[VERSION]`

## Release Notes

GitHub Release created with:
- Title: `pot-o-validator v0.2.0`
- Body: Lists all updated downstream versions

## Troubleshooting Matrix

| Problem | Check | Fix |
|---------|-------|-----|
| Workflow doesn't trigger | Tag format `pot-o-validator-v*` | Push correct tag |
| Auth fails | Secret `TRIBEWAREZ_PAT` exists | Create secret |
| Build fails | Cargo.toml valid | Fix dependencies |
| Push fails | Token has `repo` scope | Update token |
| Version mismatch | Cargo.toml in workspace | Sync versions |

## Environment Variables

Set in all jobs:

```yaml
RUST_BACKTRACE: 1
CARGO_TERM_COLOR: always
```

## Files Structure

```
.github/
├── workflows/
│   └── upstream-release.yml          # Main workflow (900+ lines)
├── scripts/
│   ├── update-dependencies.sh        # Helper for updates
│   ├── validate-workflow.sh          # Validation checks
│   └── setup-workflow.sh             # Interactive setup
├── UPSTREAM_RELEASE.md               # Full documentation
├── SETUP_GUIDE.md                    # Setup instructions
└── QUICK_REFERENCE.md                # This file
```

## Release Checklist

- [ ] All code committed and tested
- [ ] Cargo.toml versions bumped (if new release)
- [ ] CHANGELOG.md updated (optional)
- [ ] Secret `TRIBEWAREZ_PAT` configured
- [ ] Create tag: `git tag pot-o-validator-v0.2.0`
- [ ] Push tag: `git push origin pot-o-validator-v0.2.0`
- [ ] Monitor workflow: `gh run list`
- [ ] Verify downstream repos updated
- [ ] Review GitHub Release notes

## Dependency Graph

```
pot-o-core
  ├─ no dependencies on workspace crates

ai3-lib
  ├─ pot-o-core

pot-o-mining
  ├─ pot-o-core
  └─ ai3-lib

pot-o-extensions
  ├─ pot-o-core
  ├─ ai3-lib
  └─ pot-o-mining
```

## Git Tags Created

After successful run, these tags are created:

```
pot-o-core-v0.2.0
ai3-lib-v0.2.0
pot-o-mining-v0.2.0
pot-o-extensions-v0.2.0
```

## Validation Commands

```bash
# Check workflow syntax
./.github/scripts/validate-workflow.sh

# Test locally
cargo build --release
cargo test --release

# Check versions
grep '^version' Cargo.toml
grep '^version' core/Cargo.toml
grep '^version' ai3-lib/Cargo.toml
grep '^version' mining/Cargo.toml
grep '^version' extensions/Cargo.toml
```

## Manual Override

If automated workflow fails, manually update repos:

```bash
# For each downstream repo
cd pot-o-core
sed -i 's/pot-o-core = ".*"/pot-o-core = "0.2.0"/' Cargo.toml
cargo update
cargo test --release
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): update to match upstream"
git tag pot-o-core-v0.2.0
git push origin pot-o-core-v0.2.0
```

## Support & Documentation

- Full docs: `.github/UPSTREAM_RELEASE.md`
- Setup guide: `.github/SETUP_GUIDE.md`
- Workflow file: `.github/workflows/upstream-release.yml`
- Helper scripts: `.github/scripts/*.sh`

## Related Links

- [GitHub Actions Docs](https://docs.github.com/en/actions)
- [Semantic Versioning](https://semver.org/)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

