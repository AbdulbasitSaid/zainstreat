use argon2::password_hash::PasswordHasher;
use argon2::Argon2;
use sqlx::postgres::PgPoolOptions;

fn arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let Some(email) = arg_value("--email") else {
        eprintln!("usage: create_admin --email <email> [--name <name>]");
        std::process::exit(1);
    };
    let name = arg_value("--name").unwrap_or_else(|| "Admin".to_string());

    let password = rpassword::prompt_password("Password: ").expect("failed to read password");
    let confirm = rpassword::prompt_password("Confirm password: ").expect("failed to read password");
    if password != confirm {
        eprintln!("passwords did not match");
        std::process::exit(1);
    }
    if password.is_empty() {
        eprintln!("password must not be empty");
        std::process::exit(1);
    }

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    let password_hash = Argon2::default()
        .hash_password(password.as_bytes())
        .expect("failed to hash password")
        .to_string();

    sqlx::query!(
        r#"
        INSERT INTO users (name, email, password_hash, role)
        VALUES ($1, $2, $3, 'admin')
        ON CONFLICT (email) DO UPDATE
            SET password_hash = EXCLUDED.password_hash,
                name = EXCLUDED.name,
                updated_at = now()
        "#,
        name,
        email,
        password_hash,
    )
    .execute(&pool)
    .await
    .expect("failed to upsert admin user");

    println!("admin user '{email}' created/updated");
}
