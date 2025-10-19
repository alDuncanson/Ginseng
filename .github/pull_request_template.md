## ⚠️ Important: Read Before Submitting

**Ginseng is currently NOT accepting feature pull requests.** This is a passion project I work on during my free time, and I'm focusing on core development without managing contributions right now.

**Only submit this PR if it falls into one of these categories:**
- [ ] **Documentation fix** (typos, clarifications, small improvements)
- [ ] **Security fix** (see [SECURITY.md](../SECURITY.md) for reporting process)
- [ ] **Critical bug fix** (something is broken and needs immediate attention)

If this is a feature request or enhancement, please close this PR and consider opening a [discussion](https://github.com/alDuncanson/ginseng/discussions) instead. See [CONTRIBUTING.md](../CONTRIBUTING.md) for more details about the current development approach.

---

## What does this PR do?

<!-- Briefly describe what this pull request changes -->

## Type of change

- [ ] Documentation improvement
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] Security fix
- [ ] Other (please explain):

## Related issue

<!-- If this fixes an issue, reference it here with "Fixes #123" -->

## Testing

<!-- Describe how you tested these changes -->

- [ ] I have tested this change locally
- [ ] This change doesn't require testing (documentation only)

## Technical checklist

If this includes code changes:

**Rust code:**
- [ ] `cargo fmt` passes without warnings
- [ ] `cargo clippy` passes without warnings  
- [ ] Tests pass (`cargo test`)
- [ ] Follows async/await patterns consistently
- [ ] Uses proper error handling via `anyhow`

**TypeScript/React code:**
- [ ] `bun x tsc --noEmit` passes without errors
- [ ] Follows functional programming patterns
- [ ] Uses React hooks appropriately

**General:**
- [ ] Commit messages are clear and descriptive
- [ ] Changes are focused and atomic
- [ ] Referenced any related issues

## Context and philosophy

Ginseng exists to democratize peer-to-peer technologies and provide tools for digital sovereignty. This PR should align with those goals and not introduce unnecessary dependencies or centralization.

- [ ] This change supports user agency and peer-to-peer principles
- [ ] This change doesn't introduce new dependencies on centralized services
- [ ] This change maintains or improves security and privacy

## Additional notes

<!-- Any other context, concerns, or considerations for this PR -->

---

**Note:** As this is a volunteer project I work on during free time, review timelines may vary. Thank you for understanding and for contributing to free and open-source peer-to-peer technology!