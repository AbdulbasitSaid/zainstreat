use api::build_app;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json = std::env::var("LOG_FORMAT").map(|f| f != "pretty").unwrap_or(true);

    if json {
        tracing_subscriber::fmt().json().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    std::panic::set_hook(Box::new(|info| {
        tracing::error!(panic = %info, "panicked");
    }));
}

#[tokio::main]
async fn main() {
    init_tracing();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        tracing::error!("DATABASE_URL must be set");
        std::process::exit(1);
    });

    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "failed to connect to Postgres");
            std::process::exit(1);
        }
    };

    if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
        tracing::error!(%error, "failed to run database migrations");
        std::process::exit(1);
    }

    let app = build_app(pool);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%error, port, "failed to bind listener");
            std::process::exit(1);
        }
    };

    tracing::info!(port, "api listening");

    if let Err(error) = axum::serve(listener, app).await {
        tracing::error!(%error, "server error");
        std::process::exit(1);
    }
}
