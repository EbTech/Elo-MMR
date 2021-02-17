use crate::immut_database::ImmutableSportDatabase;
use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    min: Option<i32>,
    max: Option<i32>,
}

#[tracing::instrument(
    name = "Requesting the number of players whose ratings are within a range",
    skip(form, database)
)]
pub async fn request_count(
    form: web::Form<FormData>,
    database: web::Data<ImmutableSportDatabase>,
) -> Result<HttpResponse, HttpResponse> {
    let min = form.min.unwrap_or(i32::MIN);
    let max = form.max.unwrap_or(i32::MAX);
    let count = count_from_database(min, max, &database)
        .await
        .map_err(|e| HttpResponse::BadRequest().body(e))?;

    Ok(HttpResponse::Ok().json(&count))
}

#[tracing::instrument(name = "Obtaining the range count from the database", skip(database))]
pub async fn count_from_database(
    min: i32,
    max: i32,
    database: &ImmutableSportDatabase,
) -> Result<usize, String> {
    if min > max {
        return Err(format!("min={} is greater than max={}", min, max));
    }
    Ok(database.count_rating_range(min, max))
}
