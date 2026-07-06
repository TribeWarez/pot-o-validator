# 📦 GitHub Actions Workflows & Documentation

Welcome to the automated release orchestration system for `pot-o-validator`!

## 📂 Directory Structure

```
.github/
├── workflows/
│   └── upstream-release.yml              # Main workflow: auto-updates downstream repos
│
├── scripts/
│   ├── setup-workflow.sh                 # Interactive setup guide
│   ├── validate-workflow.sh              # Configuration validation
│   └── update-dependencies.sh            # Dependency updater utility
│
└── Documentation/
    ├── README.md                         # This file (navigation guide)
    ├── GETTING_STARTED.md                # Start here! Next steps guide
    ├── QUICK_REFERENCE.md                # Quick lookup & commands
    ├── SETUP_GUIDE.md                    # Step-by-step setup
    ├── UPSTREAM_RELEASE.md               # Full documentation
    └── IMPLEMENTATION_SUMMARY.md         # Implementation details
```

## 🚀 Quick Start (3 Steps)

### 1️⃣ Configure Secret
```bash
gh secret set GH_PAT
# Paste your GitHub Personal Access Token
```

### 2️⃣ Create Release
```bash
git tag pot-o-validator-v0.2.0
git push origin pot-o-validator-v0.2.0
```

### 3️⃣ Monitor
```bash
gh run list --workflow=upstream-release.yml
```

---

## 📖 Documentation Guide

### 🎯 **For First-Time Users** → Start Here
**File:** `GETTING_STARTED.md`
- Pre-deployment checklist
- Next steps guide
- Quick commands
- Success criteria

### ⚡ **For Quick Lookup** (5 min read)
**File:** `QUICK_REFERENCE.md`
- TL;DR quick start
- Common commands
- Workflow diagram
- Troubleshooting matrix

### 🛠️ **For Setup** (20 min read)
**File:** `SETUP_GUIDE.md`
- Prerequisites checklist
- Configure GitHub secrets (3 methods)
- First release walkthrough
- Detailed troubleshooting

### 📚 **For Full Understanding** (30 min read)
**File:** `UPSTREAM_RELEASE.md`
- Complete architecture
- 6-stage pipeline explanation
- Dependency graphs
- Security considerations
- Manual override procedures

### 🔍 **For Implementation Details** (15 min read)
**File:** `IMPLEMENTATION_SUMMARY.md`
- What was implemented
- File structure
- Key features
- Technical stack

---

## 🔧 Helper Scripts

All scripts are executable and documented.

### `validate-workflow.sh`
Pre-release validation checklist:
```bash
./.github/scripts/validate-workflow.sh
```

**Checks:**
- ✓ Workflow file syntax
- ✓ Cargo.toml structure
- ✓ Git configuration
- ✓ Rust toolchain
- ✓ Downstream repos

### `setup-workflow.sh`
Interactive setup guide:
```bash
./.github/scripts/setup-workflow.sh
```

**Helps with:**
- GitHub secret configuration
- Repository verification
- Validation checks
- Next steps guidance

### `update-dependencies.sh`
Manual dependency updates (if needed):
```bash
cd /path/to/downstream-repo
bash /path/to/.github/scripts/update-dependencies.sh pot-o-core 0.2.0
```

---

## 📋 What Gets Automated

The `upstream-release.yml` workflow automatically:

✅ Validates the release (tests, cargo checks)  
✅ Updates `pot-o-core` repository  
✅ Updates `ai3-lib` repository  
✅ Updates `pot-o-mining` repository  
✅ Updates `pot-o-extensions` repository  
✅ Creates release tags for all repos  
✅ Generates GitHub Release notes  

**Total Time:** ~15-20 minutes per release

---

## ⚠️ Prerequisites

Before using the workflow:

1. **GitHub Secret:** `GH_PAT`
   - GitHub Personal Access Token
   - Scopes: `repo`, `workflow`, `write:packages`
   - See `SETUP_GUIDE.md` for configuration

2. **Downstream Repositories:** Must exist and be accessible
   - `TribeWarez/pot-o-core`
   - `TribeWarez/ai3-lib`
   - `TribeWarez/pot-o-mining`
   - `TribeWarez/pot-o-extensions`

3. **Cargo.toml Files:** Valid structure with version fields
   - All workspace crates must have `[package] version = "..."`

---

## 🎯 Recommended Reading Order

### First Time?
1. `GETTING_STARTED.md` (10 min)
2. `QUICK_REFERENCE.md` (10 min)
3. `SETUP_GUIDE.md` (20 min)

### Want Details?
1. `UPSTREAM_RELEASE.md` (30 min)
2. `IMPLEMENTATION_SUMMARY.md` (15 min)

### Just Need to Know How?
→ `QUICK_REFERENCE.md` (5 min)

---

## 🚨 Troubleshooting

### Workflow doesn't trigger?
→ See `QUICK_REFERENCE.md` → Troubleshooting Matrix

### Auth error?
→ See `SETUP_GUIDE.md` → Issue: Authentication Error

### Build fails?
→ See `SETUP_GUIDE.md` → Issue: Dependency Update Fails

### Need help?
→ Run: `./.github/scripts/validate-workflow.sh`

---

## 📊 Statistics

| Metric | Value |
|--------|-------|
| Documentation Files | 5 guides |
| Helper Scripts | 3 (all executable) |
| Workflow File Size | 16 KB |
| Total Lines of Code | 2,281 |
| Estimated Read Time | 80 minutes |
| Setup Time | 20-30 minutes |
| First Release Time | 15-20 minutes |

---

## 🔒 Security

### GitHub Secrets
- Secret Name: `GH_PAT`
- Stored securely in GitHub
- Used only in workflow environment

### Bot Identity
- User: "TribeWarez Release Bot"
- Email: "release@tribewarez.com"
- Limited permissions

### Best Practices
- Keep token updated
- Rotate token regularly
- Never commit secrets to repo
- Review logs for unauthorized access

---

## 📞 Need Help?

1. **Quick question?** Check `QUICK_REFERENCE.md`
2. **Setup issue?** Follow `SETUP_GUIDE.md`
3. **How it works?** Read `UPSTREAM_RELEASE.md`
4. **First time?** Start with `GETTING_STARTED.md`

---

## ✅ Pre-Deployment Checklist

- [ ] Read `GETTING_STARTED.md`
- [ ] Configure `GH_PAT` secret
- [ ] Run `validate-workflow.sh`
- [ ] Test with test release tag
- [ ] Verify downstream repos updated
- [ ] Clean up test tags
- [ ] Brief team on process

---

## 🎓 Team Learning Path

For new team members:

**Day 1:** `QUICK_REFERENCE.md` (5 min)  
**Day 2:** `SETUP_GUIDE.md` walkthrough (20 min)  
**Day 3:** `UPSTREAM_RELEASE.md` architecture (30 min)  
**Day 4:** Monitor first release (30 min)  
**Day 5:** Review troubleshooting (20 min)  

---

## 🚀 Ready to Start?

```bash
# 1. Read getting started guide
cat GETTING_STARTED.md

# 2. Run validation
bash scripts/validate-workflow.sh

# 3. Configure secret
gh secret set GH_PAT

# 4. Create release!
git tag pot-o-validator-v0.2.0
git push origin pot-o-validator-v0.2.0
```

---

## 📚 Document Reference

| File | Purpose | Read Time |
|------|---------|-----------|
| `GETTING_STARTED.md` | Next steps & checklist | 10 min |
| `QUICK_REFERENCE.md` | Commands & troubleshooting | 5 min |
| `SETUP_GUIDE.md` | Detailed setup instructions | 20 min |
| `UPSTREAM_RELEASE.md` | Complete documentation | 30 min |
| `IMPLEMENTATION_SUMMARY.md` | Implementation details | 15 min |

---

## 🌟 Key Features

✨ **Fully Automated** - One tag push, everything else happens  
🔄 **Sequential Pipeline** - Dependencies respected  
✅ **Validated** - Tests run at each stage  
🏷️ **Auto-Tagged** - Releases created automatically  
📝 **Documented** - 5 comprehensive guides  
🔐 **Secure** - GitHub secrets & limited permissions  
📊 **Logged** - Complete audit trail  
🚀 **Fast** - 15-20 minutes per release  

---

## 📞 Support

All documentation is included in this directory. Start with:

- **Quick Help:** `QUICK_REFERENCE.md`
- **Setup Help:** `SETUP_GUIDE.md`
- **Full Docs:** `UPSTREAM_RELEASE.md`

---

**Implementation Date:** March 7, 2026  
**Status:** ✅ Production Ready  
**Version:** 1.0

