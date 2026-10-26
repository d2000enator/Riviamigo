//! Versioned private planning/alert extension. No Rivian commands or external
//! notification accounts are connected by this module.
pub mod engine;
pub mod model;
pub mod store;
#[cfg(test)]
mod tests;
pub mod weekly;
mod weekly_insights;
pub mod weekly_model;
#[cfg(test)]
mod weekly_tests;

use crate::errors::AppError;
use sqlx::PgPool;
use std::sync::atomic::{AtomicBool, Ordering};
static READY: AtomicBool = AtomicBool::new(false);
pub fn ensure_ready() -> Result<(), AppError> {
    if READY.load(Ordering::Acquire) {
        Ok(())
    } else {
        Err(AppError::DependencyUnavailable("Personal assistant is starting or unavailable. The existing vehicle dashboards remain available.".into()))
    }
}
pub async fn initialize(pool: &PgPool) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(0x5249564153534953_i64)
        .execute(&mut *tx)
        .await?;
    let table = sqlx::query_scalar::<_, Option<String>>(
        "SELECT to_regclass('riviamigo.personal_assistant_schema')::text",
    )
    .fetch_one(&mut *tx)
    .await?;
    if table.is_none() {
        sqlx::raw_sql(include_str!("schema_v1.sql"))
            .execute(&mut *tx)
            .await?;
    } else {
        let version = sqlx::query_scalar::<_, i32>(
            "SELECT version FROM riviamigo.personal_assistant_schema WHERE singleton=true",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if version != Some(1) {
            return Err(AppError::DependencyUnavailable(
                "Personal assistant storage version is unsupported; no data was changed.".into(),
            ));
        }
    }
    tx.commit().await?;
    READY.store(true, Ordering::Release);
    Ok(())
}
pub fn start_worker(pool: PgPool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(error) = initialize(&pool).await {
            tracing::error!(%error,"personal assistant unavailable; core dashboards continue");
            return;
        }
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let rows=sqlx::query_as::<_,(uuid::Uuid,uuid::Uuid)>("SELECT d.user_id,d.vehicle_id FROM riviamigo.personal_assistant_documents d JOIN riviamigo.users u ON u.id=d.user_id JOIN riviamigo.vehicle_memberships m ON m.vehicle_id=d.vehicle_id AND m.user_id=d.user_id WHERE NOT u.is_disabled ORDER BY d.user_id,d.vehicle_id")
                .fetch_all(&pool).await;
            match rows {
                Ok(rows) => {
                    for (user, vehicle) in rows {
                        if let Err(error) = store::evaluate_and_store(&pool, user, vehicle).await {
                            tracing::warn!(%error,"personal assistant evaluation deferred");
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(%error,"personal assistant monitor could not load saved plans")
                }
            }
        }
    })
}
