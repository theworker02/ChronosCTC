# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.6.x   | ✅ |
| < 0.6   | ❌ (experimental / pre-release) |

## Reporting a vulnerability

**Do not open a public GitHub issue for security-sensitive reports.**

Please report vulnerabilities privately:

1. Email: **theworker02+chronosctc-security@users.noreply.github.com**  
   (or open a private [GitHub Security Advisory](https://github.com/theworker02/ChronosCTC/security/advisories/new) if you have access)
2. Include:
   - Affected crate(s) and versions
   - Reproduction steps / proof of concept (non-destructive preferred)
   - Impact assessment (confidentiality / integrity / availability)
   - Whether the issue is already public elsewhere

We aim to acknowledge reports within **72 hours** and to provide a remediation plan within **14 days** for confirmed issues affecting published crates.

## Scope

In scope:

- Memory safety / soundness bugs in published crates that can cause undefined behavior when used as documented
- Deutsch-gate / binding bypasses that accept inconsistent worldline injections
- Supply-chain issues in release artifacts (checksum mismatches, tampered binaries)
- Secrets leakage in logs, CI, or published packages

Out of scope:

- Denial-of-service via intentionally pathological CTC maps / infinite residual landscapes (expected research surface)
- Theoretical physics claims or “CTC paradox” interpretations
- Issues only present on unreleased local branches

## Preferred disclosure

Please allow time for a fix and coordinated release before public disclosure. We will credit reporters in release notes unless anonymity is requested.

## Hardening notes for integrators

- Keep `strict_deutsch = true` in production signalling paths when residual checks are available
- Pin crate versions; prefer verifying release `SHA256SUMS`
- Treat Genesis / cosmos law sealing as trusted configuration — do not expose unconstrained law vectors to untrusted input
