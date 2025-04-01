use sqlx::types::Uuid;
use sqlx::{Error, PgPool};
use tonic::{Response, Status};

use crate::{AgentCommand, Command, service::backend::AgentCommandSender};

use super::backend::{Send, Recv};
use super::kueater::data::types;
use super::kueater::{Empty, data::activity::*};

async fn tally(
    pg_pool: &PgPool,
    sender: &AgentCommandSender,
    user_id: Uuid
) -> Result<(), Status> {
    
    let TALLY_TH: i32 = 10;

    // Add to tally
    match sqlx::query_as::<_, (bool,)> (
        "SELECT kueater.tally($1, $2)"
    ).bind(user_id).bind(TALLY_TH).fetch_one(pg_pool).await { 
        Ok((b,)) => {
            if b {
                sender.send(
                    AgentCommand { 
                        msg: Command::Recommend { user_id: user_id.to_string() }, tx: None 
                    }
                ).await.unwrap();
                sqlx::query("SELECT kueater.reset_tally($1)").bind(user_id).execute(pg_pool).await.unwrap();
            }
        },
        Err(_) => {
            return Err(Status::aborted("no tally"));
        }
     }

    Ok(())
}

pub async fn like_item(
    pg_pool: &PgPool,
    request: Recv<LikeItemMsg>,
    sender: &AgentCommandSender
) -> Send<Empty> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let data = request.into_inner();

    let menu_id = match data.item_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Menu id not a UUID"));
        }
    };

    match sqlx::query("SELECT kueater.toggle_like_menu($1, $2)")
    .bind(user_id).bind(menu_id).execute(pg_pool).await {
        Ok(_) => {
            tally(pg_pool, sender, user_id).await.unwrap();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    Ok(Response::new(Empty {  }))
}

pub async fn dislike_item(
    pg_pool: &PgPool,
    request: Recv<DislikeItemMsg>,
    sender: &AgentCommandSender
) -> Send<Empty> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let data = request.into_inner();

    let menu_id = match data.item_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Menu id not a UUID"));
        }
    };

    match sqlx::query("SELECT kueater.toggle_dislike_menu($1, $2)")
    .bind(user_id).bind(menu_id).execute(pg_pool).await {
        Ok(_) => {
            tally(pg_pool, sender, user_id).await.unwrap();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    Ok(Response::new(Empty {  }))
}

pub async fn save_item(
    pg_pool: &PgPool,
    request: Recv<SaveItemMsg>,
    sender: &AgentCommandSender
) -> Send<Empty> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let data = request.into_inner();

    let menu_id = match data.item_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Menu id not a UUID"));
        }
    };

    match sqlx::query("SELECT kueater.toggle_save_menu($1, $2)")
    .bind(user_id).bind(menu_id).execute(pg_pool).await {
        Ok(_) => {
            tally(pg_pool, sender, user_id).await.unwrap();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    Ok(Response::new(Empty {  }))
}

pub async fn like_stall(
    pg_pool: &PgPool,
    request: Recv<LikeStallMsg>,
    sender: &AgentCommandSender
) -> Send<Empty> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let data = request.into_inner();

    let stall_id = match data.item_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Stall id not a UUID"));
        }
    };

    match sqlx::query("SELECT kueater.toggle_like_stall($1, $2)")
    .bind(user_id).bind(stall_id).execute(pg_pool).await {
        Ok(_) => {
            tally(pg_pool, sender, user_id).await.unwrap();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    Ok(Response::new(Empty {  }))
}

pub async fn save_stall(
    pg_pool: &PgPool,
    request: Recv<SaveStallMsg>,
    sender: &AgentCommandSender
) -> Send<Empty> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let data = request.into_inner();

    let stall_id = match data.item_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Stall id not a UUID"));
        }
    };

    match sqlx::query("SELECT kueater.toggle_save_stall($1, $2)")
    .bind(user_id).bind(stall_id).execute(pg_pool).await {
        Ok(_) => {
            tally(pg_pool, sender, user_id).await.unwrap();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    Ok(Response::new(Empty {  }))
}