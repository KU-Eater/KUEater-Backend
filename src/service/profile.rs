use sqlx::PgPool;
use tonic::{Response, Status};
use uuid::Uuid;

use crate::no_impl;

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

pub async fn save_profile(
    pg_pool: &PgPool,
    request: Recv<SaveProfileRequest>
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

    let __query = format!("
            SELECT kueater.update_profile($1, '{}', '{}', '{}')
            ", data.username, data.gender, data.role);
    match sqlx::query(
        &__query
    )
    .bind(&user_id)
    .execute(pg_pool)
    .await {
        Ok(_) => { Ok(Response::new(Empty {  })) }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Update profile failed"));
        }
    }
}

pub async fn save_preferences(
    pg_pool: &PgPool,
    request: Recv<SavePreferencesRequest>
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

    let __query = format!("
            SELECT kueater.update_preferences($1, '{}', '{}', '{}', '{}', '{}')
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
        Ok(_) => { Ok(Response::new(Empty {})) }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Account creation failed"));
        }
    }
}