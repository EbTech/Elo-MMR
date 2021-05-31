use super::ApiError;
use crate::domain::UserName;
use crate::immut_database::SportDatabases;
use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    source: String,
    query: String,
    many: usize,
}

#[tracing::instrument(
    name = "Requesting candidate handles to complete a search query",
    skip(form, databases),
    fields(source = %form.source, query = %form.query, many = %form.many)
)]
pub async fn autocomplete(
    form: web::Form<FormData>,
    databases: web::Data<SportDatabases>,
) -> Result<HttpResponse, ApiError> {
    let database = databases
        .get(&form.0.source)
        .ok_or(ApiError::InvalidDatabase)?;
    let query = UserName::parse(form.0.query).map_err(ApiError::ValidationError)?;

    let suggestions = database.autocomplete(&query, form.0.many);

    Ok(HttpResponse::Ok().json(suggestions))
}
