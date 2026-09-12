//! Read-only, build-pinned integration with the Apostille Me HTTP API contract.
//!
//! The API server owns the canonical routes. This module embeds the exact
//! discovery manifest and public OpenAPI bytes, validates their structure and
//! safety metadata, and exposes documentation catalog values only. It performs
//! no network, filesystem, case-data, WebSocket, or HTTP-operation execution.

use std::collections::BTreeSet;

use serde_json::{json, Value};

pub const SCHEMA_VERSION: &str = "ore.api-docs.v1";
pub const DISCOVERY_PATH: &str = "/.well-known/api-docs";
pub const OPENAPI_PATH: &str = "/openapi.json";
pub const OPENAPI_ALIAS: &str = "/api/docs.json";
pub const DOCS_PATH: &str = "/api/docs";
pub const DOCS_ALIAS: &str = "/docs/api";
pub const API_REPOSITORY: &str = "apostille-me/apme-api";
pub const MCP_REPOSITORY: &str = "apostille-me/apme-mcp-server.rs";
pub const API_GIT_SHA: &str = "456c3fd94a93dd42b9b12213525ec9db99754a81";
pub const OPENAPI_SHA256: &str =
    "3f71046f7308a2957ec94de435ed57bf081865c97ec6e538858d26d7d72fa762";
pub const SHARED_CONTRACT_COMMIT: &str = "47e411311523013f90db98390671d683475d6c74";
pub const MAX_TOOL_OUTPUT_BYTES: usize = 32 * 1024;

const MANIFEST: &str = include_str!("../api-docs/api-docs.manifest.json");
const OPENAPI: &str = include_str!("../api-docs/apme.openapi.json");
const EXPECTED_PATH_COUNT: usize = 7;
const EXPECTED_OPERATION_COUNT: usize = 8;
const EXPECTED_EXPOSED_OPERATION_IDS: [&str; 3] =
    ["getHealth", "getReadiness", "getServiceBanner"];
const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];
const REQUIRED_MCP_TOOLS: [&str; 5] = [
    "api_docs_discover",
    "api_docs_get_openapi",
    "api_docs_validate",
    "api_docs_list_operations",
    "api_docs_describe_operation",
];
const SENSITIVE_TAGS: [&str; 3] = ["operations", "cases", "realtime"];

#[derive(Clone, Debug)]
struct Operation {
    operation_id: String,
    path: String,
    method: &'static str,
    summary: String,
    tags: Vec<String>,
    visibility: String,
    stability: String,
    mcp_expose: bool,
    mutating: bool,
    path_parameters: Option<Value>,
    openapi: Value,
}

impl Operation {
    fn summary_value(&self) -> Value {
        json!({
            "operationId": self.operation_id,
            "method": self.method.to_ascii_uppercase(),
            "path": self.path,
            "summary": self.summary,
            "tags": self.tags,
            "visibility": self.visibility,
            "stability": self.stability,
            "mcpExpose": self.mcp_expose,
            "mutating": self.mutating,
        })
    }

    fn detail_value(&self) -> Value {
        json!({
            "operationId": self.operation_id,
            "method": self.method.to_ascii_uppercase(),
            "path": self.path,
            "summary": self.summary,
            "tags": self.tags,
            "visibility": self.visibility,
            "stability": self.stability,
            "mcpExpose": self.mcp_expose,
            "mutating": self.mutating,
            "pathParameters": self.path_parameters,
            "openapiOperation": self.openapi,
            "execution": {
                "available": false,
                "reason": "The baseline API-docs MCP contract describes operations but does not execute HTTP requests."
            },
            "caseDataAccess": {
                "available": false,
                "reason": "This documentation catalog never reads applicant, document, case, metric, or case-event data."
            }
        })
    }
}

#[derive(Clone, Debug)]
struct ValidationReport {
    openapi_version: String,
    path_count: usize,
    operation_count: usize,
    exposed_operation_ids: Vec<String>,
}

fn valid_root_relative_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.chars().any(|character| {
            matches!(character, '?' | '#' | '\\' | '\r' | '\n' | '\t' | '\0')
                || character.is_control()
        })
}

fn lower_hex(value: &str, lengths: &[usize]) -> bool {
    lengths.contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn pointer_str<'a>(
    value: &'a Value,
    pointer: &str,
    error: &'static str,
) -> Result<&'a str, &'static str> {
    value.pointer(pointer).and_then(Value::as_str).ok_or(error)
}

fn pointer_bool(value: &Value, pointer: &str, error: &'static str) -> Result<bool, &'static str> {
    value.pointer(pointer).and_then(Value::as_bool).ok_or(error)
}

fn manifest_value() -> Result<Value, &'static str> {
    serde_json::from_str(MANIFEST).map_err(|_| "api_docs_manifest_invalid_json")
}

fn openapi_value() -> Result<Value, &'static str> {
    serde_json::from_str(OPENAPI).map_err(|_| "api_docs_openapi_invalid_json")
}

fn validate_manifest(manifest: &Value) -> Result<(), &'static str> {
    if pointer_str(
        manifest,
        "/schemaVersion",
        "api_docs_manifest_missing_schema_version",
    )? != SCHEMA_VERSION
    {
        return Err("api_docs_manifest_schema_version_mismatch");
    }
    if pointer_str(
        manifest,
        "/public/openapi/path",
        "api_docs_manifest_missing_openapi_path",
    )? != OPENAPI_PATH
    {
        return Err("api_docs_manifest_openapi_path_mismatch");
    }
    let openapi_aliases = manifest
        .pointer("/public/openapi/aliases")
        .and_then(Value::as_array)
        .ok_or("api_docs_manifest_missing_openapi_aliases")?;
    if !openapi_aliases
        .iter()
        .any(|alias| alias.as_str() == Some(OPENAPI_ALIAS))
    {
        return Err("api_docs_manifest_openapi_alias_missing");
    }
    if pointer_str(
        manifest,
        "/public/ui/path",
        "api_docs_manifest_missing_ui_path",
    )? != DOCS_PATH
    {
        return Err("api_docs_manifest_ui_path_mismatch");
    }
    let ui_aliases = manifest
        .pointer("/public/ui/aliases")
        .and_then(Value::as_array)
        .ok_or("api_docs_manifest_missing_ui_aliases")?;
    if !ui_aliases
        .iter()
        .any(|alias| alias.as_str() == Some(DOCS_ALIAS))
    {
        return Err("api_docs_manifest_ui_alias_missing");
    }
    if pointer_str(
        manifest,
        "/public/openapi/sha256",
        "api_docs_manifest_missing_openapi_sha256",
    )? != OPENAPI_SHA256
    {
        return Err("api_docs_manifest_openapi_sha256_mismatch");
    }
    if pointer_str(
        manifest,
        "/mcp/repository",
        "api_docs_manifest_missing_mcp_repository",
    )? != MCP_REPOSITORY
    {
        return Err("api_docs_manifest_mcp_repository_mismatch");
    }
    if pointer_str(manifest, "/mcp/mode", "api_docs_manifest_missing_mcp_mode")? != "read-only" {
        return Err("api_docs_manifest_mcp_mode_mismatch");
    }
    if pointer_str(
        manifest,
        "/provenance/sourceRepository",
        "api_docs_manifest_missing_source_repository",
    )? != API_REPOSITORY
    {
        return Err("api_docs_manifest_source_repository_mismatch");
    }
    let git_sha = pointer_str(
        manifest,
        "/provenance/gitSha",
        "api_docs_manifest_missing_git_sha",
    )?;
    if git_sha != API_GIT_SHA || !lower_hex(git_sha, &[40, 64]) {
        return Err("api_docs_manifest_git_sha_mismatch");
    }
    if pointer_bool(
        manifest,
        "/internal/available",
        "api_docs_manifest_missing_internal_availability",
    )? {
        return Err("api_docs_manifest_internal_docs_unexpected");
    }

    let tools = manifest
        .pointer("/mcp/tools")
        .and_then(Value::as_array)
        .ok_or("api_docs_manifest_missing_mcp_tools")?;
    let tool_names = tools
        .iter()
        .map(|tool| tool.as_str().ok_or("api_docs_manifest_invalid_mcp_tool"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if REQUIRED_MCP_TOOLS
        .iter()
        .any(|required| !tool_names.contains(required))
    {
        return Err("api_docs_manifest_required_mcp_tool_missing");
    }

    Ok(())
}

fn operation_catalog(openapi: &Value) -> Result<Vec<Operation>, &'static str> {
    let openapi_version = pointer_str(openapi, "/openapi", "api_docs_openapi_missing_version")?;
    if !openapi_version.starts_with("3.1.") {
        return Err("api_docs_openapi_version_mismatch");
    }
    let paths = openapi
        .get("paths")
        .and_then(Value::as_object)
        .ok_or("api_docs_openapi_missing_paths")?;
    if paths.len() != EXPECTED_PATH_COUNT {
        return Err("api_docs_openapi_path_count_mismatch");
    }

    let mut operation_ids = BTreeSet::new();
    let mut operations = Vec::new();
    for (path, path_item) in paths {
        if !valid_root_relative_path(path) || path.starts_with("/internal/") {
            return Err("api_docs_openapi_invalid_public_path");
        }
        let path_item = path_item
            .as_object()
            .ok_or("api_docs_openapi_invalid_path_item")?;
        let path_parameters = path_item.get("parameters").cloned();
        for method in HTTP_METHODS {
            let Some(operation) = path_item.get(method) else {
                continue;
            };
            let operation_object = operation
                .as_object()
                .ok_or("api_docs_openapi_invalid_operation")?;
            let operation_id = operation_object
                .get("operationId")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty() && value.len() <= 128)
                .ok_or("api_docs_openapi_invalid_operation_id")?;
            if !operation_ids.insert(operation_id.to_owned()) {
                return Err("api_docs_openapi_duplicate_operation_id");
            }
            let summary = operation_object
                .get("summary")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty() && value.len() <= 300)
                .ok_or("api_docs_openapi_invalid_summary")?;
            let tags = operation_object
                .get("tags")
                .and_then(Value::as_array)
                .ok_or("api_docs_openapi_invalid_tags")?
                .iter()
                .map(|tag| {
                    tag.as_str()
                        .filter(|value| !value.is_empty() && value.len() <= 64)
                        .map(str::to_owned)
                        .ok_or("api_docs_openapi_invalid_tag")
                })
                .collect::<Result<Vec<_>, _>>()?;
            if tags.is_empty() {
                return Err("api_docs_openapi_tags_empty");
            }
            let visibility = operation_object
                .get("x-ore-visibility")
                .and_then(Value::as_str)
                .ok_or("api_docs_openapi_missing_visibility")?;
            if visibility != "public" {
                return Err("api_docs_openapi_internal_operation_exposed");
            }
            let stability = operation_object
                .get("x-ore-stability")
                .and_then(Value::as_str)
                .ok_or("api_docs_openapi_missing_stability")?;
            if !matches!(stability, "stable" | "beta" | "experimental") {
                return Err("api_docs_openapi_invalid_stability");
            }
            let mcp_expose = operation_object
                .get("x-ore-mcp-expose")
                .and_then(Value::as_bool)
                .ok_or("api_docs_openapi_missing_mcp_exposure")?;
            let mutating = operation_object
                .get("x-ore-mcp-mutating")
                .and_then(Value::as_bool)
                .ok_or("api_docs_openapi_missing_mutation_classification")?;
            let expected_mutating = !matches!(method, "get" | "head" | "options");
            if mutating != expected_mutating {
                return Err("api_docs_openapi_mutation_classification_mismatch");
            }
            if mutating && mcp_expose {
                return Err("api_docs_openapi_mutation_exposed_to_mcp");
            }
            if mcp_expose
                && tags
                    .iter()
                    .any(|tag| SENSITIVE_TAGS.contains(&tag.as_str()))
            {
                return Err("api_docs_openapi_sensitive_operation_exposed_to_mcp");
            }
            let responses = operation_object
                .get("responses")
                .and_then(Value::as_object)
                .ok_or("api_docs_openapi_missing_responses")?;
            if responses.is_empty() {
                return Err("api_docs_openapi_responses_empty");
            }

            operations.push(Operation {
                operation_id: operation_id.to_owned(),
                path: path.to_owned(),
                method,
                summary: summary.to_owned(),
                tags,
                visibility: visibility.to_owned(),
                stability: stability.to_owned(),
                mcp_expose,
                mutating,
                path_parameters: path_parameters.clone(),
                openapi: operation.clone(),
            });
        }
    }

    operations.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
    if operations.len() != EXPECTED_OPERATION_COUNT {
        return Err("api_docs_openapi_operation_count_mismatch");
    }
    let exposed_operation_ids = operations
        .iter()
        .filter(|operation| operation.mcp_expose && !operation.mutating)
        .map(|operation| operation.operation_id.as_str())
        .collect::<BTreeSet<_>>();
    let expected_exposed = EXPECTED_EXPOSED_OPERATION_IDS
        .into_iter()
        .collect::<BTreeSet<_>>();
    if exposed_operation_ids != expected_exposed {
        return Err("api_docs_openapi_mcp_exposed_operations_mismatch");
    }
    Ok(operations)
}

fn validated_catalog() -> Result<ValidationReport, &'static str> {
    if MANIFEST.len() > 256 * 1024 || OPENAPI.len() > 8 * 1024 * 1024 {
        return Err("api_docs_snapshot_exceeds_contract_limit");
    }
    let manifest = manifest_value()?;
    validate_manifest(&manifest)?;
    let openapi = openapi_value()?;
    let operations = operation_catalog(&openapi)?;
    let exposed_operation_ids = operations
        .iter()
        .filter(|operation| operation.mcp_expose && !operation.mutating)
        .map(|operation| operation.operation_id.clone())
        .collect::<Vec<_>>();
    Ok(ValidationReport {
        openapi_version: pointer_str(
            &openapi,
            "/openapi",
            "api_docs_openapi_missing_version",
        )?
        .to_owned(),
        path_count: openapi["paths"]
            .as_object()
            .expect("validated paths object")
            .len(),
        operation_count: operations.len(),
        exposed_operation_ids,
    })
}

fn operations() -> Result<Vec<Operation>, &'static str> {
    let manifest = manifest_value()?;
    validate_manifest(&manifest)?;
    let openapi = openapi_value()?;
    operation_catalog(&openapi)
}

/// Return the exact discovery-manifest bytes and parsed JSON after validation.
pub fn discovery_document() -> Result<(&'static str, Value), &'static str> {
    validated_catalog()?;
    Ok((MANIFEST, manifest_value()?))
}

/// Return the exact public OpenAPI bytes and parsed JSON after validation.
pub fn openapi_document() -> Result<(&'static str, Value), &'static str> {
    validated_catalog()?;
    Ok((OPENAPI, openapi_value()?))
}

/// Return deterministic provenance and safety-validation metadata.
pub fn validation_document() -> Result<Value, &'static str> {
    let report = validated_catalog()?;
    Ok(json!({
        "schemaVersion": SCHEMA_VERSION,
        "valid": true,
        "source": {
            "mode": "build-pinned",
            "apiRepository": API_REPOSITORY,
            "apiGitSha": API_GIT_SHA,
            "mcpRepository": MCP_REPOSITORY,
            "sharedContractCommit": SHARED_CONTRACT_COMMIT,
        },
        "documents": {
            "discoveryPath": DISCOVERY_PATH,
            "openapiPath": OPENAPI_PATH,
            "openapiAlias": OPENAPI_ALIAS,
            "docsPath": DOCS_PATH,
            "docsAlias": DOCS_ALIAS,
            "openapiSha256": OPENAPI_SHA256,
            "manifestBytes": MANIFEST.len(),
            "openapiBytes": OPENAPI.len(),
        },
        "openapiVersion": report.openapi_version,
        "pathCount": report.path_count,
        "operationCount": report.operation_count,
        "mcpExposedReadOnlyOperationIds": report.exposed_operation_ids,
        "checks": {
            "manifestStructure": true,
            "canonicalRoutes": true,
            "sameOrganizationPairing": true,
            "openapiStructure": true,
            "stableUniqueOperationIds": true,
            "publicVisibilityOnly": true,
            "mutationClassification": true,
            "caseDataExposed": false,
            "metricsExposed": false,
            "webSocketExposed": false,
            "mutationExecutionExposed": false,
            "compileTimeDigestPin": true,
            "digestRecomputedAtRuntime": false,
            "digestRecomputedInCi": true,
        },
        "apiExecutionAvailable": false,
        "caseDataAccessAvailable": false,
    }))
}

/// Return a normalized, filtered operation catalog.
pub fn list_operations_document(
    tag: Option<&str>,
    include_mutating: bool,
    mcp_exposed_only: bool,
) -> Result<Value, &'static str> {
    if tag.is_some_and(|value| {
        value.is_empty() || value.len() > 64 || value.chars().any(char::is_control)
    }) {
        return Err("api_docs_invalid_tag_filter");
    }
    let operations = operations()?
        .into_iter()
        .filter(|operation| include_mutating || !operation.mutating)
        .filter(|operation| !mcp_exposed_only || operation.mcp_expose)
        .filter(|operation| {
            tag.is_none_or(|wanted| operation.tags.iter().any(|value| value == wanted))
        })
        .map(|operation| operation.summary_value())
        .collect::<Vec<_>>();
    Ok(json!({
        "schemaVersion": "ore.api-docs.operations.v1",
        "source": {
            "mode": "build-pinned",
            "apiRepository": API_REPOSITORY,
            "apiGitSha": API_GIT_SHA,
            "openapiSha256": OPENAPI_SHA256,
        },
        "filters": {
            "tag": tag,
            "includeMutating": include_mutating,
            "mcpExposedOnly": mcp_exposed_only,
        },
        "count": operations.len(),
        "operations": operations,
        "apiExecutionAvailable": false,
        "caseDataAccessAvailable": false,
    }))
}

/// Describe one documented operation by exact stable `operationId`.
pub fn describe_operation_document(operation_id: &str) -> Result<Value, &'static str> {
    if operation_id.is_empty()
        || operation_id.len() > 128
        || operation_id.chars().any(char::is_control)
    {
        return Err("api_docs_invalid_operation_id");
    }
    let operation = operations()?
        .into_iter()
        .find(|operation| operation.operation_id == operation_id)
        .ok_or("api_docs_operation_not_found")?;
    Ok(json!({
        "schemaVersion": "ore.api-docs.operation.v1",
        "source": {
            "mode": "build-pinned",
            "apiRepository": API_REPOSITORY,
            "apiGitSha": API_GIT_SHA,
            "openapiSha256": OPENAPI_SHA256,
        },
        "operation": operation.detail_value(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_snapshot_validates_with_expected_counts() {
        let report = validated_catalog().expect("snapshot must validate");
        assert_eq!(report.openapi_version, "3.1.0");
        assert_eq!(report.path_count, 7);
        assert_eq!(report.operation_count, 8);
        assert_eq!(
            report.exposed_operation_ids,
            ["getHealth", "getReadiness", "getServiceBanner"]
        );
    }

    #[test]
    fn default_operation_catalog_is_system_only() {
        let payload =
            list_operations_document(None, false, true).expect("catalog must validate");
        assert_eq!(payload["count"], 3);
        let ids = payload["operations"]
            .as_array()
            .expect("operations array")
            .iter()
            .map(|operation| operation["operationId"].as_str().expect("operation ID"))
            .collect::<Vec<_>>();
        assert_eq!(ids, ["getHealth", "getReadiness", "getServiceBanner"]);
    }

    #[test]
    fn case_mutation_description_never_offers_execution_or_data_access() {
        let payload =
            describe_operation_document("createCase").expect("operation description must validate");
        assert_eq!(payload["operation"]["mutating"], true);
        assert_eq!(payload["operation"]["mcpExpose"], false);
        assert_eq!(payload["operation"]["execution"]["available"], false);
        assert_eq!(
            payload["operation"]["caseDataAccess"]["available"],
            false
        );
    }

    #[test]
    fn case_reads_metrics_and_websocket_are_not_exposed() {
        for operation_id in ["listCases", "getCase", "getMetrics", "connectCaseEvents"] {
            let payload = describe_operation_document(operation_id)
                .expect("restricted operation description must validate");
            assert_eq!(payload["operation"]["mcpExpose"], false);
        }
    }

    #[test]
    fn exact_documents_are_bounded_and_preserve_pinned_bytes() {
        assert!(MANIFEST.len() <= MAX_TOOL_OUTPUT_BYTES);
        assert!(OPENAPI.len() <= MAX_TOOL_OUTPUT_BYTES);
        assert_eq!(
            discovery_document().expect("manifest must validate").0,
            MANIFEST
        );
        assert_eq!(
            openapi_document().expect("OpenAPI must validate").0,
            OPENAPI
        );
    }

    #[test]
    fn invalid_filters_and_unknown_operations_fail_closed() {
        assert_eq!(
            list_operations_document(Some("bad\nfilter"), false, true),
            Err("api_docs_invalid_tag_filter")
        );
        assert_eq!(
            describe_operation_document("missing"),
            Err("api_docs_operation_not_found")
        );
    }
}
