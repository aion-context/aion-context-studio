//! Policy endpoints: list, info, verify (read) and create, rules, diff, commit (authoring). All
//! delegate to `studio-core`; the id is validated before any filesystem access.

use aion_context::operations::{FileInfo, VerificationReport};
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use studio_core::audit::{self, AuditView};
use studio_core::authoring::{self, CommitInfo};
use studio_core::diff::{diff_lines, DiffLine};
use studio_core::evaluate::{self, Decision, Scalar};
use studio_core::multisig_store::{self, MultiSigProgress};
use studio_core::workspace::PolicyId;
use studio_core::{policies, PolicySummary};

use crate::app::AppState;
use crate::error::ApiResult;

/// `GET /api/policies` — every policy with a summary + current validity.
pub async fn list(State(st): State<AppState>) -> ApiResult<Json<Vec<PolicySummary>>> {
    Ok(Json(policies::list(&st.ws)?))
}

/// `GET /api/policies/{id}` — full file info (versions + signatures).
pub async fn info(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<FileInfo>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(policies::info(&st.ws, &id)?))
}

/// `GET /api/policies/{id}/verify` — the four-guarantee verification report.
pub async fn verify(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<VerificationReport>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(policies::verify(&st.ws, &id)?))
}

#[derive(Deserialize)]
pub struct CreateReq {
    pub id: String,
    pub rules: String,
}

/// `POST /api/policies` — create a new policy (genesis), signed by the operator.
pub async fn create(
    State(st): State<AppState>,
    Json(req): Json<CreateReq>,
) -> ApiResult<Json<CommitInfo>> {
    let id = PolicyId::new(&req.id)?;
    Ok(Json(authoring::create(&st.ws, &id, &req.rules)?))
}

#[derive(Serialize)]
pub struct RulesResp {
    pub rules: String,
}

/// `GET /api/policies/{id}/rules` — the current rules text.
pub async fn rules(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<RulesResp>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(RulesResp {
        rules: authoring::current_rules(&st.ws, &id)?,
    }))
}

#[derive(Deserialize)]
pub struct DiffReq {
    pub proposed: String,
}

/// `POST /api/policies/{id}/diff` — line diff of current rules vs a proposed edit (preview).
pub async fn diff(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<DiffReq>,
) -> ApiResult<Json<Vec<DiffLine>>> {
    let id = PolicyId::new(&id)?;
    let current = authoring::current_rules(&st.ws, &id)?;
    Ok(Json(diff_lines(&current, &req.proposed)))
}

#[derive(Deserialize)]
pub struct CommitReq {
    pub rules: String,
    pub message: String,
}

/// `POST /api/policies/{id}/versions` — commit a new version, signed by the operator.
pub async fn commit(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CommitReq>,
) -> ApiResult<Json<CommitInfo>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(authoring::commit(
        &st.ws,
        &id,
        &req.rules,
        &req.message,
    )?))
}

/// `GET /api/policies/{id}/multisig` — K-of-N governance progress for the current version.
pub async fn multisig(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<MultiSigProgress>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(multisig_store::progress(&st.ws, &id)?))
}

#[derive(Deserialize)]
pub struct SetMultisigReq {
    pub threshold: u32,
    pub signers: Vec<u64>,
}

/// `PUT /api/policies/{id}/multisig` — set the M-of-N policy.
pub async fn set_multisig(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SetMultisigReq>,
) -> ApiResult<Json<MultiSigProgress>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(multisig_store::set_config(
        &st.ws,
        &id,
        req.threshold,
        req.signers,
    )?))
}

#[derive(Deserialize)]
pub struct ApproveReq {
    pub author: u64,
}

/// `GET /api/policies/{id}/audit` — the recorded operation history.
pub async fn audit(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AuditView>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(audit::read(&st.ws, &id)?))
}

#[derive(Deserialize)]
pub struct ComplianceQuery {
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

/// `GET /api/policies/{id}/compliance` — a compliance report (markdown by default).
pub async fn compliance(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ComplianceQuery>,
) -> ApiResult<impl IntoResponse> {
    let id = PolicyId::new(&id)?;
    let report = audit::compliance_report(
        &st.ws,
        &id,
        q.framework.as_deref().unwrap_or("generic"),
        q.format.as_deref().unwrap_or("markdown"),
    )?;
    Ok((
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        report,
    ))
}

#[derive(Deserialize)]
pub struct ExportQuery {
    #[serde(default)]
    pub format: Option<String>,
}

/// `GET /api/policies/{id}/export` — download the file as JSON/YAML/CSV.
pub async fn export(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let id = PolicyId::new(&id)?;
    let fmt = q.format.as_deref().unwrap_or("json");
    let data = audit::export_policy(&st.ws, &id, fmt)?;
    let (ctype, disp) = match fmt {
        "yaml" => (
            "application/yaml",
            "attachment; filename=\"aion-export.yaml\"",
        ),
        "csv" => (
            "text/csv; charset=utf-8",
            "attachment; filename=\"aion-export.csv\"",
        ),
        _ => (
            "application/json",
            "attachment; filename=\"aion-export.json\"",
        ),
    };
    Ok((
        [
            (header::CONTENT_TYPE, ctype),
            (header::CONTENT_DISPOSITION, disp),
        ],
        data,
    ))
}

#[derive(Deserialize)]
pub struct SimulateReq {
    pub input: BTreeMap<String, Scalar>,
}

/// `POST /api/policies/{id}/simulate` — evaluate a proposed action against the current rules.
pub async fn simulate(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SimulateReq>,
) -> ApiResult<Json<Decision>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(evaluate::simulate(&st.ws, &id, &req.input)?))
}

/// `POST /api/policies/{id}/multisig/approve` — record an approval by an authorized signer.
pub async fn approve(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ApproveReq>,
) -> ApiResult<Json<MultiSigProgress>> {
    let id = PolicyId::new(&id)?;
    Ok(Json(multisig_store::approve(&st.ws, &id, req.author)?))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use studio_core::{policies, seed, Workspace};

    /// No read endpoint may surface a signing-key secret. Seed a workspace, read the persisted
    /// operational secret, and assert it appears in none of the serialized read responses.
    #[test]
    fn read_responses_never_leak_a_signing_key() {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("studio-api-{}-{n}", std::process::id()));
        let ws = Workspace::new(&dir);
        seed::ensure_seeded(&ws).unwrap();

        let secret = std::fs::read_to_string(ws.keys_dir().join("author-1.key")).unwrap();
        assert_eq!(secret.len(), 64, "operational key is 32 bytes hex");

        let id = studio_core::PolicyId::new("refund-authorization").unwrap();
        let bodies = [
            serde_json::to_string(&policies::list(&ws).unwrap()).unwrap(),
            serde_json::to_string(&policies::info(&ws, &id).unwrap()).unwrap(),
            serde_json::to_string(&policies::verify(&ws, &id).unwrap()).unwrap(),
        ];
        for body in &bodies {
            assert!(
                !body.contains(&secret),
                "a read response leaked the signing key"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
