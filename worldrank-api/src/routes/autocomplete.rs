use crate::domain::UserName;
use crate::immut_database::{ImmutableSportDatabase, SportDatabases};
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
) -> Result<HttpResponse, HttpResponse> {
    let database = databases
        .get(&form.0.source)
        .ok_or(HttpResponse::BadRequest())?;
    let query = UserName::parse(form.0.query).map_err(|e| {
        tracing::error!("Bad username: {:?}", e);
        HttpResponse::BadRequest().finish()
    })?;

    let suggestions = autocomplete_from_database(&query, form.0.many, database)
        .await
        .map_err(|e| HttpResponse::BadRequest().body(e))?;

    Ok(HttpResponse::Ok().json(suggestions))
}

#[tracing::instrument(
    name = "Obtaining autocomplete suggestions from the database",
    skip(query, database),
    fields(query = %query.as_ref())
)]
pub async fn autocomplete_from_database(
    query: &UserName,
    many: usize,
    database: &ImmutableSportDatabase,
) -> Result<Vec<String>, String> {
    /*if many > 200 {
        return Err(format!(
            "Requested {} players. Please limit your requests to 200.",
            many
        ));
    }*/
    Ok(database.autocomplete(query, many))
}
