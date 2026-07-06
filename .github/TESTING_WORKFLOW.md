# Testing the Upstream Release Workflow

## Current Status

✅ **Workflow files created and ready**
- `.github/workflows/upstream-release.yml` — Main workflow
- `.github/scripts/*.sh` — Helper scripts  
- `.github/*.md` — Complete documentation

## Why No Runs Show Yet

The workflow triggers on **git tag push**, not on regular commits. To see the workflow run, you need to:

1. Push the `.github/` directory to the repository (if not done yet)
2. Create a release tag matching the pattern: `pot-o-validator-v*`

## Testing the Workflow - Step by Step

### Step 1: Ensure .github is Committed

First, verify that the `.github/` directory has been committed and pushed:

```bash
cd /home/oz/RustroverProjects/pot-o-validator

# Check if .github is tracked by git
git ls-files .github/ | head -5

# Output should show workflow files like:
# .github/workflows/upstream-release.yml
# .github/scripts/setup-workflow.sh
# etc.
```

If nothing shows, commit and push:

```bash
git add .github/
git commit -m "feat: add upstream release workflow"
git push origin main
```

### Step 2: Create a Test Release Tag

```bash
# Create an annotated tag (recommended)
git tag -a pot-o-validator-v0.1.0-test -m "Test release for workflow validation"

# Or lightweight tag
git tag pot-o-validator-v0.1.0-test

# Push the tag to trigger the workflow
git push origin pot-o-validator-v0.1.0-test
```

### Step 3: Monitor the Workflow

Wait 10-30 seconds for GitHub to detect the tag, then check:

```bash
# List recent runs
gh run list --workflow=upstream-release.yml

# Expected output:
# STATUS  TITLE                    WORKFLOW                                       HEAD BRANCH  RUN NUMBER  CREATED
# ✓       Upstream Release...      Upstream Release - Update Downstream Repos     main         1           2026-03-07

# Watch in real-time
gh run watch <RUN_ID>

# View logs
gh run view <RUN_ID> --log
```

### Step 4: Verify Workflow Execution

The workflow will run through these stages:

1. **validate** — Extracts versions, runs cargo build/test
2. **update-pot-o-core** — Updates pot-o-core repo
3. **update-ai3-lib** — Updates ai3-lib repo
4. **update-pot-o-mining** — Updates pot-o-mining repo
5. **update-pot-o-extensions** — Updates pot-o-extensions repo
6. **verify-and-notify** — Creates GitHub Release

### Step 5: Clean Up Test Tag

After successful test, remove the test tag:

```bash
# Delete local tag
git tag -d pot-o-validator-v0.1.0-test

# Delete remote tag
git push origin --delete pot-o-validator-v0.1.0-test
```

---

## Expected Outcomes

### ✅ Successful Workflow Run

If all goes well, you'll see:

1. **GitHub Actions Shows 6 Completed Jobs**
   - validate: ✓
   - update-pot-o-core: ✓
   - update-ai3-lib: ✓
   - update-pot-o-mining: ✓
   - update-pot-o-extensions: ✓
   - verify-and-notify: ✓

2. **Downstream Repositories Updated** (if they exist and are accessible)
   - New commits in each repo with message: `chore(deps): update deps...`
   - New release tags created for each repo
   - Updated `Cargo.toml` and `Cargo.lock` files

3. **GitHub Release Created**
   - Release notes with all updated versions
   - Timeline of completed stages

### ⚠️ Expected Failures (First Test)

The workflow will likely fail at the update stages because:

- The downstream repositories may not exist yet or aren't accessible
- The `TRIBEWAREZ_PAT` secret may not be configured
- The workflow will try to push to repos and fail gracefully

**This is normal!** The validate stage will succeed, proving the workflow is working.

---

## Troubleshooting Test Run

### Problem: Tag push doesn't trigger workflow

**Possible causes:**

1. **Tag format is wrong**
   ```bash
   # WRONG
   git tag v0.1.0-test                    # Missing 'pot-o-validator-'
   git tag pot-o-validator-0.1.0          # Missing 'v' after name
   
   # CORRECT
   git tag pot-o-validator-v0.1.0         # Matches pattern 'pot-o-validator-v*'
   git tag pot-o-validator-v0.1.0-rc1     # Also matches pattern
   ```

2. **Tag not pushed**
   ```bash
   # Tag created but not pushed
   git tag pot-o-validator-v0.1.0
   # Push the tag!
   git push origin pot-o-validator-v0.1.0
   ```

3. **Workflow file not committed**
   ```bash
   # Verify .github/workflows/upstream-release.yml is in git
   git ls-files .github/workflows/upstream-release.yml
   
   # If not, commit it:
   git add .github/
   git commit -m "Add workflow"
   git push origin main
   ```

### Problem: Workflow runs but validate stage fails

**Check the logs:**

```bash
# View validate job logs
gh run view <RUN_ID> --log | grep -A 20 "validate"

# Common issues:
# - Cargo.lock validation failed
# - Build error in workspace
# - Test failure
```

### Problem: Update stages fail

**This is expected in first test if:**

- Downstream repos aren't accessible
- `TRIBEWAREZ_PAT` secret isn't configured
- Downstream repos don't exist

These failures are graceful — the workflow will report them in the logs.

---

## Quick Test Commands

```bash
# 1. Verify .github is committed
git ls-files .github/ | wc -l
# Should show ~11 files

# 2. Create test tag
git tag pot-o-validator-v0.1.0-test
git push origin pot-o-validator-v0.1.0-test

# 3. Check workflow (wait ~30 seconds)
gh run list --workflow=upstream-release.yml

# 4. Watch execution
gh run watch <RUN_ID>

# 5. Clean up
git push origin --delete pot-o-validator-v0.1.0-test
git tag -d pot-o-validator-v0.1.0-test
```

---

## Next Steps

### For Production Release

When ready for real release:

```bash
# 1. Configure secret (if not done)
gh secret set TRIBEWAREZ_PAT

# 2. Update versions in Cargo.toml files
sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml
sed -i 's/version = "0.1.0"/version = "0.2.0"/' core/Cargo.toml
sed -i 's/version = "0.1.0"/version = "0.2.0"/' ai3-lib/Cargo.toml
sed -i 's/version = "0.1.0"/version = "0.2.0"/' mining/Cargo.toml
sed -i 's/version = "0.1.0"/version = "0.2.0"/' extensions/Cargo.toml

# 3. Commit version changes
git add */Cargo.toml Cargo.toml
git commit -m "chore(release): bump versions to 0.2.0"
git push origin main

# 4. Create and push release tag
git tag -a pot-o-validator-v0.2.0 -m "Release v0.2.0"
git push origin pot-o-validator-v0.2.0

# 5. Monitor
gh run list --workflow=upstream-release.yml
```

---

## Documentation References

See these files in `.github/` for detailed information:

- **`.github/QUICK_REFERENCE.md`** — Quick commands and patterns
- **`.github/SETUP_GUIDE.md`** — Detailed setup with troubleshooting
- **`.github/UPSTREAM_RELEASE.md`** — Complete architecture docs
- **`.github/GETTING_STARTED.md`** — Comprehensive next steps

---

## Summary

✅ **Workflow is ready to use**

1. The `.github/` directory with all files is in the repository
2. Create a test tag: `git tag pot-o-validator-v0.1.0-test`
3. Push it: `git push origin pot-o-validator-v0.1.0-test`
4. Watch: `gh run watch`
5. Check logs for validation

The workflow will execute and you'll see all stages in GitHub Actions!

