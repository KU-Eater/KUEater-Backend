use sqlx::postgres::{PgPool, PgPoolOptions};
use std::{env, error::Error, fmt};

#[derive(Debug)]
struct DatabaseConnectionError(String);

impl fmt::Display for DatabaseConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DatabaseConnectionError encountered: {}", self.0)
    }
}
impl Error for DatabaseConnectionError {}

pub async fn connect(db_url: String) -> Result<PgPool, Box<dyn std::error::Error>> {
    
    let production: bool = match env::var("PRODUCTION") {
        Ok(_) => true,
        Err(_) => false
    };

    let mut connect_opts: PgPoolOptions = PgPoolOptions::default();
    
    if production {
        // Check if MAX_CONNECTIONS var defined
        let v = match env::var("MAX_CONNECTIONS") {
            Ok(v) => v,
            Err(_) => return Err(Box::new(DatabaseConnectionError(String::from(
                "MAX_CONNECTIONS is not defined"
            ))))
        };

        // Check if MAX_CONNECTIONS is uint32
        let n: u32 = match v.parse::<u32>() {
            Ok(v) => v,
            Err(_) => {
                return Err(Box::new(DatabaseConnectionError(String::from(
                    "Cannot parse MAX_CONNECTIONS variable into an unsigned integer"
                ))));
            }
        };
        connect_opts = connect_opts.max_connections(n);
    }

    Ok(connect_opts.connect(db_url.as_str()).await?)

}