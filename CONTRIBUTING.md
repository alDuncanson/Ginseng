# Contributing to Ginseng

## Current Development Status

Ginseng is currently in an **exploratory development phase** as I work through fundamental questions about the project's direction, branding, and technical vision. This is a passion project that I work on in my free time, and I'm taking time to ensure it develops thoughtfully.

**I am not accepting feature pull requests at this time.**

## Why This Approach?

Building thoughtful software takes time and focus. Since this is a passion project I work on during my free time, I need space to work through core decisions about direction and architecture without the overhead of managing contributions. While Ginseng is being developed in public to democratize access to peer-to-peer tools and knowledge, there's no guarantee that broader contributions will be accepted in the future—the project's direction may change or I may choose to keep it as a solo effort.

I need space to:
- Work through the core vision and ensure technical decisions align with user needs
- Establish clean architectural patterns for peer-to-peer networking
- Develop consistent branding and messaging
- Iterate on user experiences that feel intuitive and trustworthy
- Build solid cryptographic foundations with care

## How You Can Help

While I'm not accepting feature PRs, there are valuable ways to contribute:

### Bug Reports
Found something broken? Please open an issue with:
- Clear reproduction steps
- Expected vs actual behavior  
- System information (OS, version, etc.)
- Logs or error messages if available

I'll address bug reports as time permits while working on the project in my spare time.

### Security Issues
Security is paramount for a tool focused on user sovereignty. This repository has private vulnerability reporting enabled—use the **Security** tab to report vulnerabilities directly and privately. See our [Security Policy](SECURITY.md) for full details. Do not open public issues for security concerns.

### Documentation Improvements
Small fixes to documentation, typos, or clarifications may be accepted through pull requests, though I can't guarantee review timelines given this is volunteer work.

### Community Building
- Share Ginseng with others who value digital sovereignty
- Engage thoughtfully in discussions about peer-to-peer technology
- Help articulate why user agency matters in our current technological landscape
- Participate in [GitHub Discussions](https://github.com/alDuncanson/ginseng/discussions) to share ideas and feedback

### Testing and Feedback
- Try development builds and share your experience
- Provide feedback on user interface decisions
- Test edge cases and unusual network conditions
- Share your thoughts in [GitHub Discussions](https://github.com/alDuncanson/ginseng/discussions)

Keep in mind that feedback may influence development, but there's no commitment to implement suggestions since this remains a personal project.

## Technical Standards

If you do submit a PR for documentation or bug fixes, please ensure:

**Rust Code**:
- Run `cargo fmt` and `cargo clippy` without warnings
- Include tests for any logic changes
- Follow async/await patterns consistently
- Use comprehensive error handling via `anyhow`

**TypeScript/React**:
- Pass `bun x tsc --noEmit` without errors
- Follow functional programming patterns
- Use React hooks appropriately

**Commits**:
- Write clear, descriptive commit messages
- Keep changes focused and atomic
- Reference issue numbers when applicable

## Long-term Vision

The future of community contributions to Ginseng is uncertain. I may eventually open up broader collaboration, or I might choose to keep this as a personal project indefinitely. The vision and direction could evolve significantly as I work through fundamental questions about what this tool should be.

If community contributions do become welcome in the future, I'll update this document accordingly. Until then (or if that time never comes), I appreciate your understanding of this approach.

## Philosophy in Practice

Ginseng exists to give people better tools for connecting and sharing, free from corporate gatekeepers. Every technical decision should serve that goal. Your respect for this development approach helps maintain the focus needed to explore what this project could become, even if the final vision differs from current expectations.