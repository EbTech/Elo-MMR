use super::ApiError;
use crate::domain::UserName;
use crate::immut_database::SportDatabases;
use actix_web::{web, HttpResponse};
use anyhow::Context;

#[derive(serde::Deserialize)]
pub struct FormData {
    source: String,
    handle: String,
}

#[tracing::instrument(
    name = "Requesting a player's history",
    skip(form, databases),
    fields(source = %form.source, handle = %form.handle)
)]
pub async fn request_player(
    form: web::Form<FormData>,
    databases: web::Data<SportDatabases>,
) -> Result<HttpResponse, ApiError> {
    let database = databases
        .get(&form.0.source)
        .ok_or(ApiError::InvalidDatabase)?;
    let handle = UserName::parse(form.0.handle).map_err(ApiError::ValidationError)?;

    // TODO: involves file I/O, so should probably be made async.
    let player_history = database
        .player_history(&handle)
        .context("Couldn't find history for the requested player")?;

    Ok(HttpResponse::Ok().json(player_history))
}
