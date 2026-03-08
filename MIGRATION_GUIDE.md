# Migration Guide: v0.1.x → v0.2.0

## Overview

This guide explains how to upgrade from pot-o-validator v0.1.x to v0.2.0. **The v0.2.0 release will introduce significant architectural improvements** including Dependency Injection services, Tensor Network integration, and comprehensive testing.

**Current Status**: v0.1.6-alpha is pre-release. v0.2.0 is in planning.

---

## What's Changing in v0.2.0

### Major Improvements

1. **Dependency Injection Architecture**
   - Service trait abstractions across all modules
   - ServiceRegistry for flexible component composition
   - Multiple implementations per service (standard + tensor-aware)

2. **Tensor Network Model Integration**
   - Quantum-inspired consensus mechanisms
   - Entropy-based difficulty adjustment
   - Device coherence factor integration

3. **Comprehensive Testing**
   - 50+ unit tests (from current 2 smoke tests)
   - 10+ integration tests covering workflows
   - Full test coverage documentation

4. **Enhanced Documentation**
   - Per-module architecture guides
   - Service trait specifications
   - Configuration examples
   - Error handling patterns

5. **Event-Driven State Tracking**
   - Events emitted for all state changes
   - Off-chain monitoring capabilities
   - Audit trail for consensus changes

### What's NOT Changing

- ✓ Core API compatibility (with migration notes)
- ✓ Block/transaction types (extended, not broken)
- ✓ Validation logic (enhanced, not changed)
- ✓ Consensus rules (augmented, not replaced)

---

## Pre-Upgrade Checklist

Before upgrading to v0.2.0:

- [ ] Currently running v0.1.5 or v0.1.6-alpha
- [ ] Have backed up validator state/configuration
- [ ] Understand Tensor Network concepts (see REALMS Part IV)
- [ ] Reviewed v0.2.0 CHANGELOG.md
- [ ] Team familiar with trait-based DI patterns
- [ ] Testing environment available for validation

---

## Upgrade Steps

### Step 1: Plan Deployment Window

v0.2.0 introduces architectural changes. Plan accordingly:

```
Phase 1: Deploy v0.2.0 validator (non-consensus-affecting)
  ↓
Phase 2: Monitor behavior (2 weeks in parallel)
  ↓
Phase 3: Gradually enable new features
  ↓
Phase 4: Full v0.2.0 production deployment
```

### Step 2: Update Dependencies

```toml
# In your Cargo.toml
[dependencies]
pot-o-validator = "0.2.0"       # Upgrade from 0.1.x
pot-o-core = "0.2.0"            # Matches validator
pot-o-mining = "0.2.0"
ai3-lib = "0.2.0"
pot-o-extensions = "0.2.0"
```

### Step 3: Review Breaking Changes

**No breaking changes to core APIs**, but note these enhancements:

#### Service Architecture Changes
- All validators now use trait-based services
- ServiceRegistry required for instantiation
- Configuration format extended (backward compatible)

```rust
// v0.1.x style (still works)
let validator = SimpleValidator::new(config);

// v0.2.0 style (recommended)
let registry = ServiceRegistry::new(config);
let validator = registry.create_validator()?;
```

#### Error Handling Enhancements
- New error variants for tensor operations
- Extended `TribeError` enum
- Additional validation errors (all documented)

#### Configuration Extensions
- New `tensor_config` section (optional)
- New `service_registry` settings
- All new fields have sensible defaults

### Step 4: Update Configuration

Create/update `config.toml`:

```toml
# Existing v0.1.x configuration (still works)
[consensus]
difficulty_initial = 1000
difficulty_adjustment_period = 100
max_pool_size = 128

# NEW v0.2.0 configuration (optional)
[tensor]
enabled = false              # Start with false
entropy_weight = 0.5
bond_dimension = 2
max_pool_size = 128

[service_registry]
use_tensor_aware = false     # Gradual enablement
```

### Step 5: Deploy v0.2.0 Binary

```bash
# Build new validator
cargo build --release

# Backup current state
cp -r /path/to/validator/state /path/to/validator/state.backup

# Stop current v0.1.x validator
systemctl stop pot-o-validator

# Deploy v0.2.0
cp target/release/pot-o-validator /usr/local/bin/

# Start with legacy configuration
systemctl start pot-o-validator
```

### Step 6: Verify Compatibility

```bash
# Check validator starts successfully
pot-o-validator --version
# Expected: pot-o-validator 0.2.0

# Verify HTTP API responds
curl http://localhost:8080/health
# Expected: 200 OK

# Monitor logs for errors
journalctl -u pot-o-validator -f

# Validate existing state loads correctly
# Check: No migration errors
# Check: Device registry intact
# Check: Challenge queue functional
```

### Step 7: Monitor Parallel Operation

Run v0.2.0 alongside v0.1.x for 2 weeks:

```bash
# v0.1.x continues on port 8080
# v0.2.0 runs on port 8081 (different)

# Monitor both:
curl http://localhost:8080/health    # v0.1.x
curl http://localhost:8081/health    # v0.2.0

# Compare behavior:
# - Same block validation results?
# - Same transaction processing?
# - Device registry synchronized?
# - Challenge generation consistent?
```

### Step 8: Gradually Enable Features

After 2-week parallel validation:

```toml
# Update config.toml
[service_registry]
use_tensor_aware = false     # Still legacy

[tensor]
enabled = false              # Not yet
```

Enable per-feature (one at a time):

```toml
# Week 1: Enable tensor service registry
[service_registry]
use_tensor_aware = true      # Enables trait-based services

# Monitor:
# - All services instantiate correctly
# - No performance regression
# - Error handling consistent
```

```toml
# Week 2: Enable tensor network features
[tensor]
enabled = true               # Enables entropy calculations
entropy_weight = 0.5         # Gradual weighting

# Monitor:
# - Entropy calculations working
# - Difficulty adjustments stable
# - No consensus changes
```

### Step 9: Full Production Deployment

After successful monitoring and gradual enablement:

```bash
# Decommission v0.1.x validator
systemctl stop pot-o-validator-v0.1.x

# Switch v0.2.0 to primary port (8080)
# Update configuration for production

# Restart v0.2.0
systemctl restart pot-o-validator
```

---

## Configuration Reference

### v0.1.x Configuration (Still Supported)

```toml
[consensus]
difficulty_initial = 1000
difficulty_adjustment_period = 100
max_pool_size = 128
min_block_time_ms = 5000

[mining]
mml_threshold = 100
neural_path_max_distance = 256
```

### v0.2.0 New Configuration

```toml
# Service Registry Configuration
[service_registry]
use_tensor_aware = false              # bool
implementation = "standard"           # "standard" | "tensor_aware"

# Tensor Network Configuration
[tensor]
enabled = false                       # bool, enable/disable all tensor features
s_max = 1000000                       # u64, max entropy (1e6 scale)
bond_dimension = 2                    # u64, quantum bond dimension
entropy_weight = 0.5                  # f64, 0.0-1.0 entropy weighting
max_pool_size = 128                   # u64, max entangled miners

# Difficulty Adjustment Configuration
[difficulty]
tensor_adjustment = false             # bool, entropy-based adjustment
entropy_factor = 0.1                  # f64, entropy impact on difficulty
```

---

## Validation Checklist

### Immediately After Upgrade

- [ ] Validator starts without errors
- [ ] HTTP API responds to requests
- [ ] Device registry intact (all devices present)
- [ ] Challenge queue functional
- [ ] Previous transaction history preserved
- [ ] No data corruption in state

### After 2-Week Parallel Operation

- [ ] v0.2.0 validates same blocks as v0.1.x
- [ ] Transaction processing identical results
- [ ] No consensus divergence
- [ ] Error rates stable (no new errors)
- [ ] Performance within acceptable range

### After Feature Enablement

- [ ] Tensor services instantiate correctly
- [ ] Entropy calculations functioning
- [ ] Difficulty adjustments stable
- [ ] Device coherence factors applied
- [ ] Event system logging changes

---

## Troubleshooting

### Issue: "Validator fails to start with v0.2.0"

**Cause**: Configuration incompatibility

**Solution**:
```toml
# Ensure all v0.2.0 config sections have defaults
# OR use minimal config:
[consensus]
difficulty_initial = 1000

[service_registry]
use_tensor_aware = false    # Use legacy mode
```

### Issue: "State loading fails / migration error"

**Cause**: State format incompatibility (rare, v0.2.0 extends not breaks)

**Solution**:
```bash
# Restore from backup
cp /path/to/validator/state.backup /path/to/validator/state

# Try with legacy mode
systemctl start pot-o-validator  # with use_tensor_aware = false
```

### Issue: "Block validation results differ between v0.1.x and v0.2.0"

**Cause**: Likely configuration mismatch or unintended feature enablement

**Solution**:
```toml
# Ensure v0.2.0 uses legacy implementations
[service_registry]
use_tensor_aware = false

[tensor]
enabled = false
```

### Issue: "Performance degradation after upgrade"

**Cause**: Tensor features enabled without optimization

**Solution**:
1. Disable tensor features (return to legacy mode)
2. Check for debug builds (use release build)
3. Monitor CPU/memory usage
4. Gradual feature enablement with monitoring

### Issue: "Cannot downgrade back to v0.1.x"

**Cause**: v0.2.0 state extensions (backward compatible, but v0.1.x cannot read new fields)

**Solution**:
```bash
# Keep v0.1.x validator running in parallel longer
# Sync state from v0.1.x copy before downgrade
# OR restore from pre-upgrade backup

# Note: Downgrading not recommended. Instead:
# - Report issue to support
# - Wait for patch release
# - Rollback only if critical issue
```

---

## Rollback Plan

If v0.2.0 has critical issues:

```bash
# 1. Stop v0.2.0
systemctl stop pot-o-validator

# 2. Restore from backup
cp /path/to/validator/state.backup /path/to/validator/state

# 3. Deploy v0.1.x binary
cp target/release-v0.1.x/pot-o-validator /usr/local/bin/

# 4. Restart with v0.1.x
systemctl start pot-o-validator

# 5. Verify restoration
curl http://localhost:8080/health
```

**No data loss**: Backups preserve all state

---

## Performance Impact

### v0.2.0 with Legacy Mode (`use_tensor_aware = false`)

- **CPU**: +0-2% (minimal overhead)
- **Memory**: +5-10% (new struct fields)
- **Latency**: <1ms additional (trait dispatch)
- **Throughput**: Unchanged

### v0.2.0 with Tensor Features Enabled

- **CPU**: +10-20% (entropy calculations)
- **Memory**: +20-30% (tensor state)
- **Latency**: +5-10ms (entropy computation)
- **Throughput**: -5-10% (additional calculations)

**Recommendation**: Enable incrementally and monitor

---

## Testing in v0.2.0

### Unit Tests Added

- pot-o-core: 15 new tests
- pot-o-mining: 15 new tests
- pot-o-extensions: 10 new tests
- ai3-lib: 10 new tests
- Total: 50+ new tests

### Running Tests

```bash
# All tests
cargo test --release

# Specific module
cargo test --lib pot_o_mining --release

# With output
cargo test --release -- --nocapture
```

---

## Version Lock Strategy

Once deployed, v0.2.0 should remain stable. However:

1. **v0.2.1, v0.2.2, etc.** may have additional bug fixes
2. **v0.3.0+** may introduce new breaking changes
3. **Version pinning recommended**: `pot-o-validator = "0.2"`

---

## Support

If you encounter issues:

1. **Check CHANGELOG.md**: Known issues section
2. **Review configuration**: Ensure v0.2.0 config correct
3. **Enable debug logging**: Monitor state changes
4. **Consult documentation**: docs.tribewarez.com
5. **File issue**: GitHub issues on pot-o-validator repo

---

## Timeline Recommendations

```
Week 1-2: Upgrade to v0.2.0 in parallel environment
Week 3-4: Run v0.2.0 alongside v0.1.x with monitoring
Week 5-6: Gradual feature enablement (service registry first)
Week 7-8: Enable tensor features (entropy calculations)
Week 9+:  Full production v0.2.0 deployment
```

---

## Additional Resources

- **CHANGELOG.md**: Complete feature list per version
- **Implementation Mapping**: docs.tribewarez.com/implementation-map
- **Tensor Network Docs**: REALMS Part IV specification
- **API Documentation**: docs.rs/pot-o-validator

---

**Ready to upgrade? Start with Step 1: Plan Deployment Window**

Questions? See [SECURITY.md](SECURITY.md) for support contacts.
