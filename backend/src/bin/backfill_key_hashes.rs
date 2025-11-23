/// Backfill Script for key_lookup_hash
///
/// This script helps identify which API keys need to be regenerated.
/// Since we don't store plaintext keys (only bcrypt hashes), we cannot
/// backfill lookup_hash for existing keys. Users must regenerate their keys.
///
/// Usage:
///   cargo run --bin backfill_key_hashes
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    println!("🔗 Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("✅ Connected to database");

    // Count keys without lookup_hash
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM virtual_keys WHERE key_lookup_hash IS NULL")
            .fetch_one(&pool)
            .await?;

    let keys_needing_regeneration = count.0;

    if keys_needing_regeneration == 0 {
        println!("✅ All keys have lookup_hash populated!");
        println!("   No action needed.");
        return Ok(());
    }

    println!(
        "\n⚠️  Found {} API keys without lookup_hash",
        keys_needing_regeneration
    );
    println!("\n📋 Keys that need regeneration:");
    println!(
        "   {:<36} | {:<20} | {:<12} | {:<20}",
        "Key ID", "Name", "Key Prefix", "User ID"
    );
    println!("   {}", "-".repeat(95));

    let keys: Vec<(String, Option<String>, String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT
            id::text,
            name,
            key_prefix,
            user_id::text
        FROM virtual_keys
        WHERE key_lookup_hash IS NULL
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    for (id, name, prefix, user_id) in keys {
        println!(
            "   {} | {:<20} | {:<12} | {}",
            id,
            name.unwrap_or_else(|| "unnamed".to_string()),
            prefix,
            user_id.unwrap_or_else(|| "system".to_string())
        );
    }

    println!("\n🔄 Action Required:");
    println!("   These keys need to be regenerated to benefit from the new");
    println!("   O(1) lookup performance. The old keys will continue to work");
    println!("   but will use the slower fallback authentication method.");
    println!("\n   To regenerate:");
    println!("   1. Create new keys via the API: POST /auth/key/generate");
    println!("   2. Update your applications with the new keys");
    println!("   3. Delete the old keys via: DELETE /auth/key/{{key_id}}");
    println!("\n💡 Tip: Use the API endpoints or frontend UI to manage keys.");

    Ok(())
}
