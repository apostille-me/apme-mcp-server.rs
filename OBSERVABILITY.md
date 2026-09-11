# Observability

`apme-mcp-server` initializes OpenTelemetry explicitly through the immutable shared core. With a valid credential-free `OTEL_EXPORTER_OTLP_ENDPOINT`, the process exports traces, metrics, and logs through OTLP/gRPC. Exporter setup fails open to JSON stderr diagnostics so MCP availability never depends on the collector.

Tool calls create explicit spans and record `mcp.server.tool.calls` and `mcp.server.tool.duration` with closed `class` and `outcome` attributes. No argument, result, error body, URL, credential, user, tenant, request, session, source payload, or other high-cardinality value is recorded. Stdout remains protocol-only.
