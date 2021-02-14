use crate::immut_database::ImmutableSportDatabase;
use crate::routes::{health_check, request_player, request_top};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run(
    listener: TcpListener,
    //db_pool: PgPool, TODO: add actual database
    database: ImmutableSportDatabase,
) -> Result<Server, std::io::Error> {
    let database_ptr = web::Data::new(database);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger)
            .route("/health_check", web::get().to(health_check))
            .route("/top", web::post().to(request_top))
            .route("/player", web::post().to(request_player))
            .app_data(database_ptr.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
