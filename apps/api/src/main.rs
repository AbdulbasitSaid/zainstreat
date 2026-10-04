use aws_sdk_s3::config::{BehaviorVersion, Builder as S3ConfigBuilder, Credentials, Region};
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

fn require_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        tracing::error!("{key} must be set");
        std::process::exit(1);
    })
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

    let cookie_secure = std::env::var("COOKIE_SECURE")
        .map(|v| v == "true")
        .unwrap_or(false);

    let minio_endpoint = require_env("MINIO_ENDPOINT");
    let minio_access_key = require_env("MINIO_ACCESS_KEY");
    let minio_secret_key = require_env("MINIO_SECRET_KEY");
    let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "menu-images".into());
    let public_base_url = require_env("PUBLIC_API_URL");

    let s3_config = S3ConfigBuilder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1")) // MinIO ignores region; the SDK still requires one
        .endpoint_url(minio_endpoint)
        .credentials_provider(Credentials::new(
            minio_access_key,
            minio_secret_key,
            None,
            None,
            "minio-static",
        ))
        .force_path_style(true) // MinIO requires path-style bucket addressing
        .build();
    let client = aws_sdk_s3::Client::from_conf(s3_config);

    // Idempotent — tolerates the bucket already existing from a prior boot,
    // same "fix drift on startup" spirit as sqlx::migrate!() above.
    match client.create_bucket().bucket(&bucket).send().await {
        Ok(_) => tracing::info!(%bucket, "created media bucket"),
        Err(err)
            if err
                .as_service_error()
                .is_some_and(|e| e.is_bucket_already_owned_by_you() || e.is_bucket_already_exists()) =>
        {
            tracing::info!(%bucket, "media bucket already exists");
        }
        Err(err) => {
            tracing::error!(%err, %bucket, "failed to create media bucket");
            std::process::exit(1);
        }
    }

    let media = api::MediaConfig { client, bucket, public_base_url };

    let app = match api::build_app(pool, cookie_secure, media).await {
        Ok(app) => app,
        Err(error) => {
            tracing::error!(%error, "failed to build application");
            std::process::exit(1);
        }
    };

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
