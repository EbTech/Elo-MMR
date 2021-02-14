use crate::domain::PlayerSummary;
use crate::immut_database::ImmutableSportDatabase;
use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    many: usize,
    #[serde(default)]
    start: usize,
}

#[tracing::instrument(
    name = "Requesting top players by rank",
    skip(form, database),
    fields(
        many = %form.many,
        start = %form.start
    )
)]
pub async fn request_top(
    form: web::Form<FormData>,
    database: web::Data<ImmutableSportDatabase>,
) -> Result<HttpResponse, HttpResponse> {
    let player_summaries = top_from_database(form.0.many, form.0.start, &database)
        .await
        .map_err(|e| HttpResponse::BadRequest().body(e))?;

    Ok(HttpResponse::Ok().json(player_summaries))
}

#[tracing::instrument(name = "Obtaining the top players from the database", skip(database))]
pub async fn top_from_database(
    many: usize,
    start: usize,
    database: &ImmutableSportDatabase,
) -> Result<&[PlayerSummary], String> {
    if many > 200 {
        return Err(format!(
            "Requested {} players. Please limit your requests to 200.",
            many
        ));
    }
    let num_players = database.num_players();
    let end = num_players.min(start + many);
    database
        .index_by_rank(start..end)
        .ok_or_else(|| format!("Start index {}/{} out of bounds", start, num_players))
}
