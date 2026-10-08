# Contributing to ModelShelf

Start with [development setup](docs/development.md), [architecture](docs/architecture.md), and
[implementation status](docs/implementation-status.md). Small focused pull requests are welcome.

1. Open an issue describing the problem and expected behavior before large architectural changes.
2. Keep provider, transfer, storage and UI responsibilities separate.
3. Add regression tests for download recovery, ownership or path-security changes.
4. Use translation keys for user-facing text; update English and Turkish dictionaries together.
5. Run the documented checks. Describe any platform-specific checks you could not run.
6. Update documentation when behavior or limitations change.

Never commit tokens, real user databases, downloaded weights, or fabricated benchmark results.
Do not weaken path checks, gated-access behavior, or verification labels to make a test pass.
Use local mock servers for transport tests. A new provider should expose metadata and immutable file identity
without receiving unrestricted access to user files.

Suggested labels: `good first issue`, `help wanted`, `bug`, `enhancement`, `documentation`,
`frontend`, `rust`, `integration`, `testing`, `security`, `windows`.

By contributing, you agree your contributions are licensed under the MIT license.
