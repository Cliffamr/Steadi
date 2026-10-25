# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |
| < 0.1.0 | No        |

## Reporting a Vulnerability

If you discover a security vulnerability in Steadi, please report it responsibly.

**Email**: [siddharth@khattar.dev](mailto:siddharth@khattar.dev)

Please include:

- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### What to Expect

- **Acknowledgement** within 48 hours of your report.
- **Status update** within 7 days with an assessment and remediation timeline.
- **Credit** in the release notes (unless you prefer to remain anonymous).

### Safe Harbor

We consider security research conducted in good faith to be authorized. We will not pursue legal action against researchers who:

- Make a good-faith effort to avoid privacy violations, data destruction, or service disruption
- Report vulnerabilities promptly and provide sufficient detail to reproduce the issue
- Do not publicly disclose the vulnerability before a fix is available

## Scope

Veil Browser runs on-device. Web pages use isolated child WebViews and receive no Tauri capability or IPC access. The primary security surface areas are:

- **Remote-page isolation**: external child WebViews receive no Tauri capability or filesystem API
- **IPC boundary**: invoke commands are allowlisted only for the trusted local `main` WebView
- **Navigation policy**: non-HTTPS URLs, popups, downloads, and sensitive permissions are denied
- **Native API usage**: Windows API calls provide capture exclusion and opacity
