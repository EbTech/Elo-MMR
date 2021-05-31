use super::ApiError;
use crate::immut_database::SportDatabases;
use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    source: String,
    min: Option<i32>,
    max: Option<i32>,
}

#[tracing::instrument(
    name = "Requesting the number of players whose ratings are within a range",
    skip(form, databases),
    fields(source = %form.source)
)]
pub async fn request_count(
    form: web::Form<FormData>,
    databases: web::Data<SportDatabases>,
) -> Result<HttpResponse, ApiError> {
    let database = databases
        .get(&form.0.source)
        .ok_or(ApiError::InvalidDatabase)?;
    let min = form.min.unwrap_or(i32::MIN);
    let max = form.max.unwrap_or(i32::MAX);
    if min > max {
        let err_string = format!("min={} is greater than max={}", min, max);
        return Err(ApiError::ValidationError(err_string));
    }
    let count = database.count_rating_range(min, max);

    Ok(HttpResponse::Ok().json(&count))
}
