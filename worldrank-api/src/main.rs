use worldrank_api::configuration::get_configuration;
use worldrank_api::startup::Application;
use worldrank_api::telemetry::{get_subscriber, init_subscriber};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set up tracing telemetry.
    let subscriber = get_subscriber("info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // Get config settings.
    let configuration = get_configuration().expect("Failed to read configuration.");

    // Run application.
    let application = Application::build(&configuration).await?;
    application.run_until_stopped().await
}
