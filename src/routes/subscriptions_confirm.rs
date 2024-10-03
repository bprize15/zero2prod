use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use reqwest::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use super::error_chain_format;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String
}

#[tracing::instrument(
    name = "Confirm a pending subscriber"
    skip(parameters, pool)
)]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> Result<HttpResponse, ConfirmSubscriptionError> {
    let subscriber_id = get_subscriber_id_from_token(&pool, &parameters.subscription_token)
        .await
        .context("Failed to retrieve the subscriber id associated with the provided token.")?
        .ok_or(ConfirmSubscriptionError::InvalidTokenError)?;
    confirm_subscriber(&pool, subscriber_id)
        .await
        .context("Failed to update the subscriber status to `confirmed`.")?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(thiserror::Error)]
pub enum ConfirmSubscriptionError {
    #[error("There is no subscriber associated with the provided token.")]
    InvalidTokenError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error)
}

impl std::fmt::Debug for ConfirmSubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_format(self, f)
    }
}

impl ResponseError for ConfirmSubscriptionError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            ConfirmSubscriptionError::InvalidTokenError => StatusCode::UNAUTHORIZED,
            ConfirmSubscriptionError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_subscriber_id_from_token(pool: &PgPool, subscription_token: &str) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}