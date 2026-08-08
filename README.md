# apme-mcp-server.rs

Canonical public repository for `apostille-me/apme-mcp-server.rs`.

The server currently implements newline-delimited JSON-RPC over stdio, negotiates MCP protocol revision `2025-06-18`, and exposes a read-only `zed_dependency_graph` tool. It never writes credentials, modifies repositories, or invokes application APIs.

The product-neutral dependency-graph model, closed tool descriptor, validation, and text-plus-structured result are supplied by `ore-mcp-zed-graph`, pinned by full Git revision and committed `Cargo.lock`. Product package coordinates and repository policy remain local.

The official-`rmcp` lifecycle and final `2025-11-25` protocol migration are separate DEN-957 work and are not claimed complete by this repository-recovery change.

## Canonical Zed graph

- `apostille-me/apme-clients`
- `apostille-me/apme-interfaces`
- `apostille-me/apme-libs`
- `apostille-me/apme-cli`
- `apostille-me/apme-sync`
- `shared-auth/shared-auth-clients`

Packages materialize under `.vendor/.zed`.

## Repository delivery

The repository is live on GitHub. The initial source history was published through the authenticated recovery lane tracked by DEN-2285 and DEN-2797. There is no local `publish.sh` step to run after cloning this repository.

Subsequent changes must use a focused feature branch and reviewed pull request. Do not rewrite the initial history, force-push shared refs, or place personal access tokens in Git configuration, source, workflow inputs, logs, issues, or pull requests.

Exact recovery and rebase evidence, including the shared-graph base commit and the still-missing sibling E2E repository, is recorded in [`docs/recovery-delivery.md`](docs/recovery-delivery.md).

## Validate

```bash
cargo metadata --locked --format-version 1 >/dev/null
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

## Git submodules and Zed

A composing monorepo may retain this repository as an exact committed gitlink. Zed remains authoritative for package identity and dependency intent. Adopt a canonical existing gitlink with `zed overtake --git-submodules`; do not create a second long-name coordinate, a duplicate workspace path, or an uncommitted submodule checkout.

Tracking: `apostille-me/.github#4`, GitHub Project #1, DEN-2285, DEN-2797, DEN-957, and the `github.com/apostille-me` Linear project.
