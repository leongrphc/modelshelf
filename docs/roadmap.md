# Roadmap

The first release is an experimental Windows MVP, not a stable product announcement.
The [implementation checklist](implementation-status.md) distinguishes verified work from remaining tasks.
The [prioritized next steps](next-steps.md) list actionable follow-up work and acceptance criteria in Turkish.

## Before stable 1.0

- Complete interactive Windows acceptance testing across disconnected drives, low space and gated repositories.
- Validate very large repositories and multi-gigabyte transfers under real network interruption.
- Expand component accessibility and desktop automation coverage.
- Review filesystem race resistance and credential behavior on supported Windows versions.
- Sign Windows packages and publish maintainer-owned support/security links.
- Read-only Hugging Face cache indexing with shared-blob-aware presentation.
- Native Xet chunk transport only after protocol-specific integrity and recovery tests.

## Later

Provider adapters for Ollama/LM Studio inventory and ModelScope; Linux and macOS packaging;
advanced dependency inspection and compatibility diagnostics; optional copy-on-import;
bandwidth limits; CLI companion; portable mode; signed updates; richer translations.

No cloud backend, plugin marketplace, model training or inference engine is planned for the MVP.
