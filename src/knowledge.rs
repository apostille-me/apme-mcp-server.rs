//! Shared fleet knowledge. This is descriptive and exposes no mutation path.

use serde_json::{Value, json};

#[must_use]
pub fn shared_platform() -> Value {
    json!({
        "oreKubernetes": {
            "role": "GitOps deployment and runtime topology",
            "diagnosticsOnly": true,
            "clusterMutationExposed": false
        },
        "sharedDefinitions": {
            "role": "shared service and infrastructure contracts",
            "consumerMustPinReviewedRevision": true
        },
        "dpm": {
            "role": "declarative migration planning and verification",
            "databaseMutationExposed": false
        },
        "cloudflareSquarespace": {
            "role": "edge, DNS, and site-handoff context",
            "credentials": "environment only",
            "mutationExposed": false
        },
        "supabase": {
            "role": "data and authentication boundary where adopted",
            "credentials": "environment/header only",
            "payloadTelemetry": false
        },
        "fiducia": {
            "role": "secret and lease delivery boundary",
            "credentials": "environment/header only",
            "secretValuesExposed": false
        }
    })
}
