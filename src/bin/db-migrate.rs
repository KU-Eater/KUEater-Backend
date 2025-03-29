use std::env::var;
use sqlx::PgPool;
use kueater_backend::db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = var("DATABASE_URL").unwrap_or_else(|_| {
        panic!("Error: DATABASE_URL unset")
    });
    let pg: PgPool = db::connect(url).await.unwrap_or_else(|e| {
        panic!("Cannot create connection to database: {e:?}")
    });
    sqlx::migrate!().run(&pg).await.unwrap_or_else(|e| {
        panic!("Problem while migrating: {e:?}")
    });
    println!("Database migration success!");

    Ok(())
}