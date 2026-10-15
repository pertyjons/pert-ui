# Contributing to pert-ui

Thank you for helping improve pert-ui. The library is under development, and its
public Rust API has not yet been released on crates.io.

Before opening a pull request, read [AGENTS.md](AGENTS.md) and the relevant
[design notes](docs/architecture.md). Open an issue for a substantial API or
behavior change so its scope can be discussed first. A focused bug fix can go
directly to a pull request with a reproduction and an explanation of the change.

Use stable Rust and the toolchain pinned in `rust-toolchain.toml`. Keep code,
documentation, examples and commit messages in English. Include a test when it
checks a meaningful behavior or contract. Run the checks required by the change
table in [AGENTS.md](AGENTS.md#verification-and-review), and state which checks
ran in the pull request. Native behavior needs the platform and result recorded
in [verification](docs/verification.md#native-evidence); headless checks alone do
not establish native support.

Public API changes need rustdoc and a complete example using the public facade.
Do not add application-specific semantics to general controls. See the
[architecture](docs/architecture.md) and [contracts](docs/contracts.md) for the
library boundaries and compatibility policy.

Use GitHub issues for ordinary bugs and feature requests. Report suspected
vulnerabilities through the [security policy](SECURITY.md).
