use once_cell::sync::Lazy;
use uuid::Uuid;
use worldrank_api::configuration::get_configuration;
use worldrank_api::startup::Application;
use worldrank_api::telemetry::{get_subscriber, init_subscriber};

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    address: String,
    // pub db_pool: PgPool, TODO: upgrade to PostgreSQL
}

impl TestApp {
    pub async fn spawn() -> Self {
        // `TRACING` is only executed the first time `initialize` is invoked.
        Lazy::force(&TRACING);

        // Randomise configuration to ensure test isolation
        let configuration = {
            let mut c = get_configuration().expect("Failed to read configuration.");
            // Use a different database for each test case
            c.database.database_name = Uuid::new_v4().to_string();
            // Use a random OS port
            c.application.port = 0;
            c
        };

        // Launch the application as a background task
        let application = Application::build(&configuration)
            .await
            .expect("Failed to build application.");
        let address = format!("http://127.0.0.1:{}", application.port());
        let _ = tokio::spawn(application.run_until_stopped());

        Self { address }
    }

    pub async fn post_health_check(&self) -> reqwest::Response {
        reqwest::Client::new()
            .get(&format!("{}/health_check", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post(&self, method: &str, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/{}", &self.address, method))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_top(&self, body: String) -> reqwest::Response {
        self.post("top", body).await
    }

    pub async fn post_count(&self, body: String) -> reqwest::Response {
        self.post("count", body).await
    }

    pub async fn post_player(&self, body: String) -> reqwest::Response {
        self.post("player", body).await
    }

    pub async fn post_autocomplete(&self, body: String) -> reqwest::Response {
        self.post("autocomplete", body).await
    }
}
