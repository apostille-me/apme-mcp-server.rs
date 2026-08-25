# Apostille Me MCP Server

    Read-only MCP diagnostics for apostille document, provider, status, and delivery contracts. The server is a Rust MCP process over stdio. Stdout is exclusively the JSON-RPC wire; structured diagnostics go to stderr and optional OTLP.

    ## Tools

    - `apme_fleet_map`
- `apme_plan`
- `apme_runtime_readiness`
- `apme_shared_platform`
- `apme_lifecycle_state`
- `apme_safety_boundary`

    Every tool is read-only. Planning accepts a closed workload enum plus bounded numeric fields. The server has no arbitrary URL, command, filesystem, database, GitHub mutation, cluster mutation, or secret-value input.

    ## Product topology

    - `apme-api` — apostille workflow API
- `apme-interfaces` — canonical document and provider contracts
- `apostille-me-libs` — document-status and provider domain libraries
- `apme-sync` — offline-first workflow synchronization
- `apostille-me-infra` — Kubernetes and bounded Cloudflare edge infrastructure

    ## Security boundary

    - Jurisdiction and provider decisions require authoritative human or provider review.
- MCP never uploads documents, changes case status, or purchases delivery.
- Document contents and personal identifiers are excluded from tools and telemetry.

    The shared core is pinned at `c6101656c8227251d1dbd61df54f03a186b42ade`. It provides bounded MCP framing, explicit OTLP/gRPC traces, metrics and logs, JSON stderr diagnostics, redaction, low-cardinality tool metrics, and the formal runtime lifecycle. Each tool also owns an explicit span with `skip_all`; arguments and results are never recorded. Configuration readiness reports environment-variable presence only and performs no authentication or network request.

    This server contains no authenticated HTTP client. If a future tool adds one, it must use fixed or strictly validated HTTP(S) origins, reject credentials/query/fragment/private/metadata targets, disable redirects and ambient proxies, keep credentials in sensitive headers, cap every response, and add adversarial tests before merge.

    ## Shared platform knowledge

    The bounded `shared_platform` tool documents ORE Kubernetes, shared definitions, dpm, Cloudflare/Squarespace, Supabase, and Fiducia without exposing a mutation or credential surface.

    ## Validate

    ```sh
    cargo fmt --all -- --check
    cargo clippy --locked --all-targets --all-features -- -D warnings
    cargo test --locked --all-targets --all-features
    cargo build --locked --release
    cargo audit --deny warnings
    ```
