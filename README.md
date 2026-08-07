# apme-mcp-server.rs

Executable repository seed for the canonical public repository `apostille-me/apme-mcp-server.rs`.

The server implements newline-delimited JSON-RPC over stdio, negotiates MCP protocol revision `2025-06-18`, and exposes a read-only `zed_dependency_graph` tool. It never writes credentials, modifies repositories, or invokes application APIs.

## Canonical Zed graph

- `apostille-me/apme-clients`
- `apostille-me/apme-interfaces`
- `apostille-me/apme-libs`
- `apostille-me/apme-cli`
- `apostille-me/apme-sync`
- `shared-auth/shared-auth-clients`

Packages materialize under `.vendor/.zed`.

## Publish the repository

From this seed directory, with an authenticated GitHub CLI session:

```bash
./publish.sh
```

The publisher refuses to overwrite an existing repository, initializes one atomic source history, and pushes `main`. It does not read or embed a personal access token.

## Validate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Git submodules and Zed

A composing monorepo may retain this repository as an exact committed gitlink. Zed remains authoritative for package identity and dependency intent. Adopt a canonical existing gitlink with `zed overtake --git-submodules`; do not create a second long-name coordinate, a duplicate workspace path, or an uncommitted submodule checkout.

Tracking: `apostille-me/.github#4`, GitHub Project #1, and the `github.com/apostille-me` Linear project.
