//! User-private, session-authenticated planning records. These routes do not
//! enroll vehicles, issue vehicle commands, or send external notifications.
use crate::{
    db::vehicles::require_vehicle_session_access,
    errors::AppError,
    middleware::auth::{AppState, AuthUser},
    services::personal_assistant::{
        self,
        model::{AssistantState, Document},
        store,
    },
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/vehicles/{id}/assistant", get(get_state).put(save))
        .route("/vehicles/{id}/assistant/weekly", get(get_weekly))
        .route("/vehicles/{id}/assistant/preview", post(preview))
        .route(
            "/vehicles/{id}/assistant/alerts/{alert_id}/acknowledge",
            post(acknowledge),
        )
        .route(
            "/vehicles/{id}/assistant/receipts/{receipt_id}",
            get(get_receipt),
        )
}
pub fn upload_router() -> Router<AppState> {
    Router::new().route(
        "/vehicles/{id}/assistant/maintenance/{entry_id}/receipts",
        post(upload_receipt),
    )
}
async fn authorize(state: &AppState, auth: &AuthUser, vehicle: Uuid) -> Result<(), AppError> {
    require_vehicle_session_access(&state.pool, auth, vehicle).await?;
    personal_assistant::ensure_ready()
}
async fn get_state(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(vehicle): Path<Uuid>,
) -> Result<Json<AssistantState>, AppError> {
    authorize(&state, &auth, vehicle).await?;
    Ok(Json(
        store::state(&state.pool, auth.user_id, vehicle).await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewBody {
    document: Document,
}
async fn preview(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(vehicle): Path<Uuid>,
    Json(body): Json<PreviewBody>,
) -> Result<Json<personal_assistant::model::Evaluation>, AppError> {
    authorize(&state, &auth, vehicle).await?;
    let data = store::snapshot(&state.pool, vehicle).await?;
    let (_, _, observation) = store::document(&state.pool, auth.user_id, vehicle).await?;
    Ok(Json(
        personal_assistant::engine::evaluate(
            &body.document,
            &data,
            &observation,
            chrono::Utc::now(),
        )
        .map_err(AppError::Validation)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveBody {
    revision: i64,
    document: Document,
    #[serde(default)]
    reset_vacation: bool,
}
async fn save(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(vehicle): Path<Uuid>,
    Json(body): Json<SaveBody>,
) -> Result<Json<AssistantState>, AppError> {
    authorize(&state, &auth, vehicle).await?;
    store::save_document(
        &state.pool,
        auth.user_id,
        vehicle,
        body.revision,
        body.document,
        body.reset_vacation,
    )
    .await?;
    Ok(Json(
        store::state(&state.pool, auth.user_id, vehicle).await?,
    ))
}
async fn acknowledge(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((vehicle, alert)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    authorize(&state, &auth, vehicle).await?;
    let changed=sqlx::query("UPDATE riviamigo.personal_assistant_alerts SET acknowledged_at=COALESCE(acknowledged_at,now()) WHERE id=$1 AND user_id=$2 AND vehicle_id=$3")
        .bind(alert).bind(auth.user_id).bind(vehicle).execute(&state.pool).await?;
    if changed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({"ok":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptInput {
    filename: String,
    mime_type: String,
    data_base64: String,
}
fn receipt_bytes(body: &ReceiptInput) -> Result<Vec<u8>, AppError> {
    if body.filename.trim().is_empty()
        || body.filename.chars().count() > 200
        || body
            .filename
            .chars()
            .any(|c| c.is_control() || c == '/' || c == '\\')
        || body.data_base64.len() > 7_000_000
    {
        return Err(AppError::Validation(
            "Choose a short filename and a PDF, JPEG or PNG up to 5 MiB.".into(),
        ));
    }
    let bytes = STANDARD
        .decode(&body.data_base64)
        .map_err(|_| AppError::Validation("Receipt encoding is invalid.".into()))?;
    if bytes.is_empty() || bytes.len() > 5 * 1024 * 1024 {
        return Err(AppError::Validation(
            "Receipts must be between one byte and 5 MiB.".into(),
        ));
    }
    let valid = match body.mime_type.as_str() {
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        _ => false,
    };
    if !valid {
        return Err(AppError::Validation(
            "Receipt file content must match PDF, JPEG or PNG.".into(),
        ));
    }
    Ok(bytes)
}
async fn upload_receipt(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((vehicle, entry)): Path<(Uuid, Uuid)>,
    Json(body): Json<ReceiptInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    authorize(&state, &auth, vehicle).await?;
    let bytes = receipt_bytes(&body)?;
    let mut tx = state.pool.begin().await?;
    let doc=sqlx::query_scalar::<_,serde_json::Value>("SELECT document FROM riviamigo.personal_assistant_documents WHERE user_id=$1 AND vehicle_id=$2 FOR UPDATE")
        .bind(auth.user_id).bind(vehicle).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    let doc: Document = serde_json::from_value(doc).map_err(|e| AppError::Internal(e.into()))?;
    if !doc.maintenance.iter().any(|e| e.id == entry) {
        return Err(AppError::NotFound);
    }
    let (count,size)=sqlx::query_as::<_,(i64,i64)>("SELECT count(*),COALESCE(sum(octet_length(payload)),0)::bigint FROM riviamigo.personal_assistant_receipts WHERE user_id=$1 AND vehicle_id=$2")
        .bind(auth.user_id).bind(vehicle).fetch_one(&mut *tx).await?;
    if count >= 100 || size + bytes.len() as i64 > 100 * 1024 * 1024 {
        return Err(AppError::Validation(
            "This vehicle's receipt allowance is 100 files or 100 MiB.".into(),
        ));
    }
    let id = Uuid::new_v4();
    let digest = Sha256::digest(&bytes).to_vec();
    sqlx::query("INSERT INTO riviamigo.personal_assistant_receipts(id,user_id,vehicle_id,entry_id,filename,mime_type,payload,sha256) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(id).bind(auth.user_id).bind(vehicle).bind(entry).bind(&body.filename).bind(&body.mime_type).bind(bytes).bind(digest).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({"id":id})))
}
#[derive(Serialize)]
struct ReceiptDownload {
    filename: String,
    mime_type: String,
    data_base64: String,
    sha256: String,
}
async fn get_receipt(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((vehicle, receipt)): Path<(Uuid, Uuid)>,
) -> Result<Json<ReceiptDownload>, AppError> {
    authorize(&state, &auth, vehicle).await?;
    let (filename,mime_type,bytes,sha)=sqlx::query_as::<_,(String,String,Vec<u8>,String)>("SELECT filename,mime_type,payload,encode(sha256,'hex') FROM riviamigo.personal_assistant_receipts WHERE id=$1 AND user_id=$2 AND vehicle_id=$3")
        .bind(receipt).bind(auth.user_id).bind(vehicle).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound)?;
    Ok(Json(ReceiptDownload {
        filename,
        mime_type,
        data_base64: STANDARD.encode(bytes),
        sha256: sha,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_disguised_receipts() {
        let b = ReceiptInput {
            filename: "receipt.pdf".into(),
            mime_type: "application/pdf".into(),
            data_base64: STANDARD.encode(b"<script>bad</script>"),
        };
        assert!(receipt_bytes(&b).is_err());
    }
    #[test]
    fn accepts_small_pdf() {
        let b = ReceiptInput {
            filename: "receipt.pdf".into(),
            mime_type: "application/pdf".into(),
            data_base64: STANDARD.encode(b"%PDF-1.4\nfixture\n%%EOF"),
        };
        assert!(receipt_bytes(&b).is_ok());
    }
    #[test]
    fn rejects_path_filenames() {
        let b = ReceiptInput {
            filename: "../receipt.pdf".into(),
            mime_type: "application/pdf".into(),
            data_base64: STANDARD.encode(b"%PDF-1.4"),
        };
        assert!(receipt_bytes(&b).is_err());
    }
}

async fn get_weekly(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(vehicle): Path<Uuid>,
) -> Result<Json<personal_assistant::model::WeeklySummary>, AppError> {
    authorize(&state, &auth, vehicle).await?;
    Ok(Json(
        personal_assistant::weekly::summary(&state.pool, auth.user_id, vehicle).await?,
    ))
}
