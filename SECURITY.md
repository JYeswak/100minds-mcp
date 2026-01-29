# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do NOT** open a public GitHub issue
2. Email security@zeststream.ai with:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested fixes

## Response Timeline

- **Initial response:** Within 48 hours
- **Status update:** Within 7 days
- **Fix timeline:** Depends on severity (critical: 24-48h, high: 7 days, medium: 30 days)

## Security Measures

100minds implements several security measures:

### Cryptographic Provenance
- Ed25519 signatures on all decisions
- SHA-256 hash chain for tamper detection
- Keys stored with restrictive permissions (0600)

### No Secrets in Code
- API keys read from environment variables only
- No hardcoded credentials
- Database files excluded from version control

### Dependency Security
- Regular `cargo audit` checks
- Dependabot enabled for automated updates
- Minimal dependency footprint

## Security Best Practices for Users

1. **Protect your signing key**: The Ed25519 key at `~/.local/share/100minds/provenance.key` should be kept secure
2. **Use environment variables**: Never hardcode `ANTHROPIC_API_KEY` in scripts
3. **Database permissions**: Ensure `*.db` files have appropriate permissions

## Acknowledgments

We appreciate responsible disclosure and will acknowledge security researchers who report valid vulnerabilities (with their permission).
