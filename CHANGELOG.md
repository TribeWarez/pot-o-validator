# Changelog

All notable changes to pot-o-validator and related crates are documented in this file.

## [Unreleased]

### Planned Features (v0.2.0)
- Dependency Injection service architecture across all programs
- Tensor Network Model integration from REALMS Part IV
- Enhanced quantum-inspired consensus mechanisms
- Service registry pattern for flexible implementation switching
- Comprehensive event system for state tracking

---

## [0.1.6-alpha] - 2026-03-08

### Changed
- **CRITICAL FIX**: Resolved crates.io category slug inconsistency
  - Changed invalid slug to `cryptography::cryptocurrencies`
  - Ensures proper categorization on crates.io
  - Blocks v0.1.6 publication until resolved

### Fixed
- **Security**: Updated SECURITY.md contact email
  - Previous: security@pay.tribewarez.com
  - Updated: security@tribewarez.com
  - Ensures vulnerability reports reach proper channel

- **CI/CD**: Enhanced workflow configuration
  - Added manual trigger (`workflow_dispatch`) to publish workflow
  - Allows manual crate publishing via GitHub Actions
  - Better release control and debugging capabilities

- **Dependencies**: Updated upstream release workflow
  - Improved downstream repo synchronization
  - Better version consistency across projects

### Added
- **Documentation**: Enhanced SECURITY.md
  - Detailed security guidelines for contributors
  - Vulnerability reporting procedures
  - Security contact information

- **Funding**: Added FUNDING.yml configuration
  - Links to Patreon donation page
  - Links to PayPal donation option
  - Enables GitHub "Sponsor" button display

- **Deployment**: Added crate deployment execution workflow
  - Automated crate publication process
  - Environment configuration for publish jobs
  - CARGO_REGISTRY_TOKEN integration

### Known Issues
- v0.1.6-alpha: In-development version (not finalized)
- Multiple "Initial plan" commits in history (WIP pattern)
- Limited test coverage (2 smoke tests vs. 50+ target)
- Sparse documentation for sub-crates (recommend expansion)

### Migration Notes
- No breaking changes from v0.1.5
- All existing code compatible
- Future v0.2.0 will introduce significant enhancements
- See MIGRATION_GUIDE.md (when released) for v0.1.x → v0.2.0

---

## [0.1.5] - 2026-02-XX

### Added
- Initial stable release of pot-o-validator
- HTTP API for proof validation
- Device registry and management
- Mining challenge generation
- MML (Minimal Memory Loss) validation
- Neural path distance calculations
- TribeWarez testnet consensus support

### Components
- **pot-o-validator**: Main validator service (HTTP API, config, consensus)
- **pot-o-core**: Block/transaction types, error handling, constants
- **ai3-lib**: Tensor engine, ESP compatibility, mining operations
- **pot-o-mining**: Challenge generation, proof validation, consensus
- **pot-o-extensions**: DeFi operations, pool strategy, device protocol, bridge, networking

### Documentation
- Basic README with build instructions
- Implementation mapping reference
- API documentation links
- License information

### Known Limitations
- v0.1.5 represents baseline implementation
- No quantum-inspired features
- Basic service architecture (non-trait-based in most modules)
- Limited extensibility patterns
- Minimal test coverage

---

## [0.1.0-test], [0.1.0-test-2] - Early Testing

### Status
- Internal testing versions
- Not recommended for production use
- Superseded by v0.1.5+

### Notes
- Used for evaluation and proof-of-concept
- Multiple iterations for stability testing
- Foundation for v0.1.5 release

---

## Version Comparison Matrix

| Version | pot-o-validator | pot-o-core | pot-o-mining | ai3-lib | pot-o-extensions | Status |
|---------|-----------------|------------|--------------|---------|------------------|--------|
| 0.2.0 | Planned | 0.2.0 | 0.2.0 | 0.2.0 | 0.2.0 | ⏳ Unreleased |
| 0.1.6-alpha | 0.1.6-alpha | 0.2.0* | 0.1.6-alpha | 0.1.6-alpha | 0.1.6-alpha | 🔧 In Development |
| 0.1.5 | 0.1.5 | 0.1.5 | 0.1.5 | 0.1.5 | 0.1.5 | ✅ Stable |
| 0.1.0 | 0.1.0 | 0.1.0 | 0.1.0 | 0.1.0 | 0.1.0 | ⚠️ Deprecated |

*Note: pot-o-core version mismatch in 0.1.6-alpha (0.2.0) should be resolved before release

---

## Breaking Changes

### None between v0.1.x releases
- All v0.1.x versions maintain backward compatibility
- API surfaces unchanged
- Data formats stable
- Migration required only when moving to v0.2.0+

---

## Upgrade Paths

### v0.1.0 → v0.1.5
- **Action**: Update dependency: `pot-o-validator = "0.1.5"`
- **Breaking Changes**: None
- **Effort**: Straightforward update, no code changes required

### v0.1.5 → v0.1.6-alpha
- **Action**: Update dependency: `pot-o-validator = "0.1.6-alpha"`
- **Breaking Changes**: None
- **Changes**: Security updates, CI/CD improvements
- **Effort**: Straightforward update, no code changes required

### v0.1.x → v0.2.0 (Future)
- **Expected**: Requires migration guide (TBD)
- **Likely Changes**: Service architecture refactoring
- **Expected Impact**: Potential API changes, new trait-based services
- **Timeline**: When v0.2.0 documentation available

---

## Testing Status

### v0.1.6-alpha Current Status
- **Unit Tests**: 2 smoke tests
- **Integration Tests**: None documented
- **Test Coverage**: ~5% (estimated)
- **Blockers**: Insufficient coverage for production release

### v0.2.0 Target (Recommended Before Release)
- **Unit Tests**: 50+ tests
- **Integration Tests**: 10+ scenarios
- **Test Coverage**: 60%+ (target)
- **Components**: Full coverage of core, mining, extensions, ai3-lib

---

## Publication Status

### Ready for crates.io
- ✅ pot-o-core v0.2.0 (metadata complete, backward compatible)
- ⚠️ pot-o-validator v0.1.6-alpha (metadata complete, but alpha)
- ⚠️ pot-o-mining v0.1.6-alpha (metadata complete, but alpha)
- ⚠️ ai3-lib v0.1.6-alpha (metadata complete, but alpha)
- ⚠️ pot-o-extensions v0.1.6-alpha (metadata complete, but alpha)

### Recommended Actions Before Publication
1. Resolve pot-o-core version inconsistency (v0.2.0 in v0.1.6-alpha context)
2. Expand test coverage to 50+ tests
3. Clean commit history (replace "Initial plan" commits)
4. Create comprehensive MIGRATION_GUIDE.md for v0.1.x → v0.2.0

---

## Related Projects

- **pot-o-contractz**: Solana smart contracts for PoT-O (v0.2.0 - PUBLICATION READY)
- **Solana Programs**: tribewarez-pot-o, tribewarez-staking, tribewarez-vault, tribewarez-swap
- **Documentation**: docs.tribewarez.com

---

## Contributing

See [SECURITY.md](SECURITY.md) for security guidelines.  
See [LICENSE](LICENSE) for license information.

---

## Future Roadmap

### v0.2.0 (Major Release)
- [ ] Dependency Injection architecture
- [ ] Tensor Network Model (REALMS Part IV)
- [ ] Service registry pattern
- [ ] Comprehensive test suite (50+)
- [ ] Migration guide from v0.1.x
- [ ] Event-driven state tracking
- [ ] Enhanced documentation

### v0.2.1+
- [ ] Performance optimization
- [ ] Additional consensus mechanisms
- [ ] Extended device protocol support
- [ ] Bridge network enhancements

### Long-term (v0.3+)
- [ ] Quantum-inspired optimization algorithms
- [ ] Cross-chain interoperability
- [ ] Advanced pool strategies
- [ ] Machine learning integration

---

## Acknowledgments

PoT-O (Proof of Tensor Optimizations) is developed by the TribeWarez community.  
Tensor network concepts based on REALMS Part IV research.

---

**Changelog Maintained Since**: v0.1.5  
**Last Updated**: 2026-03-08  
**Next Update**: v0.2.0 release
