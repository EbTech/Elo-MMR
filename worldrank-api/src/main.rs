use std::net::TcpListener;
use worldrank_api::configuration::get_configuration;
use worldrank_api::immut_database::ImmutableSportDatabase;
use worldrank_api::startup::run;
use worldrank_api::telemetry::{get_subscriber, init_subscriber};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set up tracing telemetry.
    let subscriber = get_subscriber("worldrank-api".into(), "info".into());
    init_subscriber(subscriber);

    // Get config settings
    let configuration = get_configuration().expect("Failed to read configuration.");

    // Start network socket to act as API endpoint
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;

    // Start database. TODO: upgrade to PostgreSQL
    let database = ImmutableSportDatabase::new(&configuration.database.path_to_data)
        .expect("Failed to start in-memory database");
    /*let connection_pool = PgPoolOptions::new()
    .connect_timeout(std::time::Duration::from_secs(2))
    .connect_with(configuration.database.with_db())
    .await
    .expect("Failed to connect to Postgres.");*/

    // Start web app
    run(listener, database)?.await
}
