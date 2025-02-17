use std::env::var;
use sqlx::PgPool;
use kueater_backend::db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pg: PgPool = db::connect(var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&pg).await?;
    println!("Database migration success!");

    Ok(())
}