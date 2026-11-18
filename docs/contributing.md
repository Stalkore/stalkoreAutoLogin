# Contributing

[Documentation](README.md) · [Architecture](architecture.md) · [Tests](testing.md)

## Design decisions

Use SOLID, Clean Code, Clean Architecture and pragmatic programming to solve
concrete maintenance problems. Keep the stable interface small and test behavior
through it rather than adding layers to satisfy a pattern.

| Principle | Application here |
| --- | --- |
| Single responsibility | Configuration selects inputs; policy decides; the SDK adapter performs operations |
| Open/closed | Add another ClientOperations adapter without editing policy decisions |
| Substitutability | Production and mock adapters obey token, status and action-result contracts |
| Interface segregation | Policy asks only for its observations, logical operations and logging |
| Dependency inversion | Policy depends on ClientOperations and public logical types |
| Pragmatic maintenance | One maintained recovery implementation, explicit state and bounded failures |

Recovery, PIN and dispatch modules share one policy state. Preserve ordering,
reentrancy guards, native SSO completion and attempt lifetimes. A fixed two-source
configuration rule needs no registry; native provider registration belongs to
the SDK. Comments explain lifetimes, ordering and decisions that names alone
cannot express.

## Preserve operational contracts

Keep `autoLogin.dll`, `autoLogin.ini`, RO_* aliases, C exports and calling
conventions stable. Preserve the one-second action gate, per-field environment
priority and one-attempt PIN rule. New behavior needs focused tests; moving code
must pass the existing behavioral suite.

Native client profiles and address evidence belong to the private adapter.
Do not add memory access, a second hook or build-specific constants here.
SDK runtime implementations are separate from this public contract consumer.

## Verification and source hygiene

Project-owned executable code and tests use Rust. Documentation, comments and
logs use English. Run the [test matrix](testing.md) sequentially with one job;
retain caches, lockfiles and owned fixture source.

Never log credentials or derive Debug for credential-owning types. Only the
empty example is versioned. Local INI files, instructions, binaries, logs and
captures remain ignored. Audit both staged source and outgoing history before
publication. Use synthetic fixtures and keep live observations dated and
separate from source-test results.
