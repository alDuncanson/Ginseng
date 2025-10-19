# Security Policy

## Reporting Security Vulnerabilities

Security is fundamental to Ginseng's mission of providing trustworthy peer-to-peer communication. If you discover a security vulnerability, please help us address it responsibly.

### How to Report

**For security issues, please do NOT open a public GitHub issue.**

This repository has **private vulnerability reporting enabled**. To report a security vulnerability:

1. Navigate to the **Security** tab in this repository
2. Click **"Report a vulnerability"** 
3. Fill out the advisory form with as much detail as possible
4. Click **"Submit report"**

GitHub will automatically notify me and add you as a collaborator on the security advisory for coordinated disclosure.

### What to Include

While only the title and description are required, please provide as much detail as possible:

- **Title**: Clear, concise summary of the vulnerability
- **Description**: Detailed explanation of the issue and its potential impact
- **Steps to reproduce**: Clear reproduction steps
- **Affected components**: Which parts of Ginseng are affected
- **Severity assessment**: Your evaluation of the impact
- **Proof-of-concept**: Any code or demonstrations (if applicable)
- **Suggested fixes**: Ideas for mitigation or resolution (if you have them)

You can use the [GitHub Security Lab report template](https://github.com/github/securitylab/blob/main/docs/report-template.md) as a guide.

### What to Expect

When you submit a private vulnerability report:

- **Automatic notification**: GitHub automatically notifies me of your report
- **Collaborator access**: You'll be added as a collaborator on the security advisory
- **Credit**: You'll be automatically credited as the reporter (unless you prefer anonymity)
- **Response**: I'll do my best to respond when I have time to work on the project
- **Updates**: I'll provide updates as I'm able to investigate and work on fixes
- **Coordinated disclosure**: We'll work together on timing when a fix is ready

Please understand this is a passion project that I work on in my free time, so response times may vary.

### Scope

Security reports are welcomed for:

- **Core application vulnerabilities** in the Rust backend or TypeScript frontend
- **Cryptographic implementation issues** in peer-to-peer networking
- **Dependency vulnerabilities** that affect Ginseng's security
- **Build process security** issues that could affect distributed binaries
- **Configuration vulnerabilities** that could expose user data

### Out of Scope

Please don't report:
- Issues in third-party applications or services
- Social engineering attacks
- Physical security issues
- Theoretical vulnerabilities without practical exploit paths

### Security Best Practices for Users

While using Ginseng:

- **Keep updated**: Use the latest version to get security fixes
- **Network awareness**: Understand that P2P connections expose your IP address to peers
- **File permissions**: Be mindful of what files you share and with whom
- **Local security**: Protect your device with appropriate security measures

### Development Security

For those building from source or contributing:

- **Dependencies**: I try to keep dependencies updated as time permits
- **Build environment**: Use trusted build environments (the Nix flake helps ensure reproducibility)
- **Code review**: Security-relevant changes get extra attention when possible
- **Testing**: Security considerations are included in testing when feasible

## Supported Versions

As an early-stage passion project, I focus security efforts on the latest release when time permits. There's no formal support policy or guaranteed timelines for security updates.

## Security Architecture

Ginseng's security is built on several foundations:

- **End-to-end encryption** via Iroh's QUIC implementation
- **Peer authentication** through ed25519 public key cryptography  
- **No central servers** to minimize attack surface
- **Local-first design** to keep data under user control

## Responsible Disclosure

I appreciate security researchers who report vulnerabilities responsibly. As time allows while working on this passion project, I'll do my best to:

- Credit you in release notes (unless you prefer to remain anonymous)
- Work with you on disclosure timing once a fix is ready
- Keep you updated on progress as I'm able to work on issues

This is a volunteer effort in my spare time, so please be patient with response times. Thank you for helping improve Ginseng's security.