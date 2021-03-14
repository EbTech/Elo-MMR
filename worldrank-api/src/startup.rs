use crate::configuration::Settings;
use crate::immut_database::ImmutableSportDatabase;
use crate::routes::{autocomplete, health_check, request_count, request_player, request_top};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: &Settings) -> Result<Self, std::io::Error> {
        // Start database. TODO: upgrade to PostgreSQL
        let database = ImmutableSportDatabase::new(&configuration.database.path_to_data)
            .expect("Failed to start in-memory database");

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(&address)?;

        let port = listener.local_addr().unwrap().port();
        let server = run(listener, database)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn run(
    listener: TcpListener,
    //db_pool: PgPool, TODO: upgrade to PostgreSQL
    database: ImmutableSportDatabase,
) -> Result<Server, std::io::Error> {
    let database_ptr = web::Data::new(database);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger)
            .route("/health_check", web::get().to(health_check))
            .route("/top", web::post().to(request_top))
            .route("/count", web::post().to(request_count))
            .route("/player", web::post().to(request_player))
            .route("/autocomplete", web::post().to(autocomplete))
            .app_data(database_ptr.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
