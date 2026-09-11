# Security policy

    Report vulnerabilities privately to the `apostille-me` maintainers. Never include secrets, customer data, source payloads, or exploit material in a public issue.

    ## Runtime boundary

    - stdio is the only transport and stdout is the MCP wire;
    - tools are deterministic, read-only, and fail closed on unknown fields or out-of-range numbers;
    - no tool accepts arbitrary URLs, commands, source payloads, credentials, or mutation instructions;
    - readiness exposes presence booleans only;
    - telemetry excludes arguments, results, identities, secrets, and high-cardinality values.

    - Jurisdiction and provider decisions require authoritative human or provider review.
- MCP never uploads documents, changes case status, or purchases delivery.
- Document contents and personal identifiers are excluded from tools and telemetry.
