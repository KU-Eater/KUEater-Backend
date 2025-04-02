use sqlx::PgPool;
use tonic::{Response, Status};
use uuid::Uuid;

use crate::{AgentCommand, Command, service::backend::AgentCommandSender};

use super::backend::{Send, Recv};
use super::kueater::{Empty, data::*};

fn vec_to_pg_array(vec: &[String]) -> String {
    if vec.is_empty() {
        return "{}".to_string();
    }
    
    // Escape each string and join with commas
    let elements: Vec<String> = vec
        .iter()
        .map(|s| {
            // Escape quotes and handle special characters
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{}\"", escaped)
        })
        .collect();
    
    format!("{{{}}}", elements.join(","))
}

pub async fn account_readiness(
    pg_pool: &PgPool,
    request: Recv<AccountReadinessRequest>
) -> Send<AccountReadinessResponse> {

    let extensions = request.extensions().clone();
    let data = request.into_inner();

    let mut user_id = data.user_id.clone();
    if user_id.is_empty() {
        user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();
    }

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    // Account readiness means
    // Is user_id associated already has username,
    // and they have user preference?
    // We need two SQL statements
    let query = sqlx::query_as::<_, (bool,)>(
        "
        SELECT kueater.get_user_readiness($1)
        "
    ).bind(&user_id).fetch_one(pg_pool).await;

    match query {
        Ok((ready,)) => {
            return Ok(Response::new(
                AccountReadinessResponse { ready: ready }
            ));
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"));
        }
    }
}

pub async fn create_account(
    pg_pool: &PgPool,
    sender: &AgentCommandSender,
    request: Recv<CreateAccountRequest>
) -> Send<Empty> {
    let extensions = request.extensions().clone();
    let data = request.into_inner();

    let mut user_id = data.user_id.clone();
    if user_id.is_empty() {
        user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();
    }

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    // Create account
    let __query = format!("
            SELECT kueater.update_profile($1, '{}', '{}', '{}')
            ", data.name, data.gender, data.role);
    match sqlx::query(
        &__query
    )
    .bind(&user_id)
    .execute(pg_pool)
    .await {
        Ok(_) => {}
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Account creation failed"));
        }
    }

    // Create preferences
    let __query = format!("
            SELECT kueater.create_preferences($1, '{}', '{}', '{}', '{}', '{}')
        ", vec_to_pg_array(&data.dietary),
        vec_to_pg_array(&data.allergies),
        vec_to_pg_array(&data.cuisines),
        vec_to_pg_array(&data.dislikes),
        vec_to_pg_array(&data.likes)
    );

    match sqlx::query(
        &__query
    )
    .bind(&user_id)
    .execute(pg_pool)
    .await {
        Ok(_) => { 
            sender.send(
                AgentCommand { 
                    msg: Command::Recommend { user_id: user_id.to_string() }, tx: None 
                }
            ).await.unwrap();
            Ok(Response::new(Empty {})) 
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Account creation failed"));
        }
    }
}