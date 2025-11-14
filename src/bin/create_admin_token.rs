use anyhow::Result;
use api::auth::sign_admin_token;
use api::config;
use chrono::{Duration, Utc};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate admin tokens for UI/CLI access", long_about = None)]
struct Args {
    /// Token scope (e.g., "ui", "admin", "cli")
    #[arg(short, long, default_value = "ui")]
    scope: String,

    /// Expiration in days (default: 365 days)
    #[arg(short, long, default_value_t = 365)]
    days: i64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Get settings to retrieve private key
    let settings = config::get_settings();

    // Parse signing key
    let private_key_bytes = hex::decode(&settings.token_private_key)?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(
        &private_key_bytes[..]
            .try_into()
            .expect("Invalid private key length"),
    );

    // Calculate expiration
    let expiration = (Utc::now() + Duration::days(args.days)).timestamp();

    // Generate token
    let token = sign_admin_token(&args.scope, expiration, &signing_key)?;

    // Print token with prefix
    let prefixed_token = format!("admin_{}", token);

    println!("\n✅ Admin token generated successfully!\n");
    println!("Scope:      {}", args.scope);
    println!("Expires in: {} days", args.days);
    println!("\nToken:");
    println!("{}", prefixed_token);
    println!("\nUse this token in the Authorization header:");
    println!("Authorization: Bearer {}\n", prefixed_token);

    Ok(())
}
