# Security Policy

TribeWarez takes the security of the PoT-O Validator Service extremely seriously. This document outlines our security practices, reporting procedures, and supported versions.

## Reporting Security Vulnerabilities

If you discover a security vulnerability in PoT-O Validator, please report it **responsibly and confidentially**.

### ⚠️ DO NOT

- Open a public GitHub issue
- Disclose the vulnerability publicly before we have time to address it
- Post details in community channels or discussions

### ✅ DO

1. **Use GitHub's Private Security Advisory**
   - Navigate to the **Security** tab → **Report a vulnerability**
   - This creates a private discussion visible only to maintainers

2. **Or Email Security Concerns**
   - Contact: [security@tribewarez.com](mailto:security@tribewarez.com)
   - Include detailed information about the vulnerability
   - Provide steps to reproduce and proof-of-concept if possible

3. **PGP/GPG Encryption** (Optional)
   - For sensitive disclosures, you may encrypt your report using our public key
   - Request our GPG key by emailing security@tribewarez.com

## Vulnerability Scope

**In Scope** (high priority):
- Consensus mechanism flaws that could lead to invalid block acceptance
- State corruption or inconsistency bugs
- Authentication/authorization bypass vulnerabilities
- Private key exposure or cryptographic weaknesses
- RPC API vulnerabilities that expose sensitive data
- Denial-of-service attacks that crash validators
- Fund loss or theft vectors

**Out of Scope** (but appreciated):
- Issues in documentation or comments (no security impact)
- UI/UX issues in non-critical tooling
- Unverified theoretical vulnerabilities without proof-of-concept
- Issues in upstream dependencies (report directly to maintainers)

## Response Timeline

- **Acknowledgment**: Within 48 hours of report submission
- **Initial Assessment**: Within 1 week
- **Fix Development**: Timeline depends on severity (critical: days, high: weeks, medium: months)
- **Disclosure**: After patch release or 90 days from report, whichever comes first (responsible disclosure)

## Severity Levels

| Level | Impact | Timeline |
|-------|--------|----------|
| **Critical** | Consensus failure, fund loss, complete system compromise | Fix within days, patch release within 1 week |
| **High** | Partial system compromise, significant data exposure | Fix within 1-2 weeks, patch release within 1 month |
| **Medium** | Limited security impact, temporary workarounds available | Fix within 1 month, included in regular release cycle |
| **Low** | Minimal security impact, defense-in-depth improvement | Fix included in next regular release |

## Security Best Practices

### For Operators Running PoT-O Validator

1. **Keep Software Updated**
   - Subscribe to security release notifications
   - Update to latest patches immediately for critical issues
   - Test patches in staging before production deployment

2. **Network Security**
   - Restrict RPC endpoint access to trusted networks
   - Use firewalls and rate limiting
   - Disable public access to admin endpoints

3. **Key Management**
   - Store validator keys securely (hardware wallet, secure enclave)
   - Rotate keys regularly
   - Never share private keys or seed phrases
   - Use environment variables or secure secret management (never in code)

4. **Monitoring & Logging**
   - Enable security logging
   - Monitor for unusual validator behavior
   - Set up alerts for consensus anomalies
   - Maintain audit logs of all state changes

5. **Dependency Management**
   - Regularly audit Rust dependencies with `cargo audit`
   - Pin critical dependencies to tested versions
   - Review dependency updates before applying

### For Contributors

1. **Code Review Requirements**
   - Security-critical code requires at least 2 approvals
   - Cryptographic changes require expert review
   - All consensus logic changes are subject to peer review

2. **Testing Standards**
   - All security-related code must include unit tests
   - Consensus bugs must have regression tests
   - Fuzz testing recommended for parsing logic

3. **Secure Coding Guidelines**
   - Avoid unsafe code; justify with comments if necessary
   - Validate all external inputs
   - Use constant-time comparisons for sensitive values
   - Never log private keys, seeds, or sensitive data

## Supported Versions

Only the latest stable release receives security patches. We recommend always running the latest version.

| Version | Status | Security Support |
|---------|--------|------------------|
| Latest | Active | ✅ Supported |
| Older versions | Deprecated | ⚠️ Best-effort only |

## Third-Party Security Audits

PoT-O Validator is undergoing security review. Audit reports and remediation status will be published here.

## Responsible Disclosure Policy

We follow the [Open Source Security Foundation (OSSF) Responsible Disclosure Guidelines](https://securitypolicy.dev/):

- We will acknowledge receipt of vulnerability reports
- We will provide transparent updates on remediation progress
- We will credit security researchers (unless anonymity is requested)
- We will coordinate disclosure timing with stakeholders

## Security Acknowledgments

We deeply appreciate the security research community's responsible disclosure. Researchers who report vulnerabilities will be acknowledged (with permission) in:
- Release notes for the patch
- ACKNOWLEDGMENTS.md file in the repository
- Our public security advisory

## Questions?

For security questions that don't relate to a specific vulnerability, please:
- Check our documentation and FAQ
- Open a non-sensitive GitHub discussion
- Email security@tribewarez.com with [SECURITY QUESTION] in the subject

---

**Last Updated**: 2026-03-08
**Version**: 1.0
