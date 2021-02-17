use crate::domain::HistoryPoint;
use crate::domain::UserName;
use crate::immut_database::ImmutableSportDatabase;
use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    handle: String,
}

#[tracing::instrument(
    name = "Requesting a player's history",
    skip(form, database),
    fields(handle = %form.handle)
)]
pub async fn request_player(
    form: web::Form<FormData>,
    database: web::Data<ImmutableSportDatabase>,
) -> Result<HttpResponse, HttpResponse> {
    let handle = UserName::parse(form.0.handle).map_err(|e| {
        tracing::error!("Bad username: {:?}", e);
        HttpResponse::BadRequest().finish()
    })?;

    let player_history = player_from_database(&handle, &database)
        .await
        .map_err(|e| HttpResponse::BadRequest().body(e))?;

    Ok(HttpResponse::Ok().json(player_history))
}

#[tracing::instrument(
    name = "Obtaining a player's history from the database",
    skip(handle, database),
    fields(handle = %handle.as_ref())
)]
pub async fn player_from_database(
    handle: &UserName,
    database: &ImmutableSportDatabase,
) -> Result<Vec<HistoryPoint>, String> {
    // We swap in more user-friendly error messages.
    // TODO: involves file I/O, so should probably be made async.
    database.player_history(handle).map_err(|e| {
        tracing::error!("Failed to get history: {:?}", e);
        format!("Couldn't find history for {:?}", handle)
    })
}
