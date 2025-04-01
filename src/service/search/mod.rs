use std::collections::HashMap;

use crate::{AgentCommand, Command, service::backend::AgentCommandSender};

use futures::{stream, StreamExt};
use serde::Deserialize;
use sqlx::types::{Decimal, Uuid};
use sqlx::{Error, PgPool};
use tokio::sync::oneshot;
use tonic::{Response, Status};

use super::backend::{Send, Recv};
use super::kueater::data::types;
use super::kueater::{Empty, data::search::*};

#[derive(Debug, Deserialize, sqlx::FromRow)]
struct MenuItem {
    uuid: String,
    name: String,
    price: f64,
    likes: i32,
    dislikes: i32,
    stall_name: String,
    stall_lock: String,
    image_url: String,
    score: Option<f64>,
    reason: Option<String>,
    liked: bool,
    disliked: bool,
    saved: bool
}

#[derive(Debug, Deserialize, sqlx::FromRow)]
struct Stall {
    uuid: String,
    rank: i32,
    name: String,
    image_url: String,
    location: String,
    operating_hours: String,
    price_range: String,
    tags: String,
    reviews: i32,
    likes: i32,     // Aggregate from menu likes
    rating: f32,
    saved: bool
}

pub async fn search(
    pg_pool: &PgPool,
    sender: &AgentCommandSender,
    request: Recv<SearchRequest>
) -> Send<SearchResponse> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let data = request.into_inner();

    if data.query.is_empty() { return Err(Status::invalid_argument("Search query is empty")) }

    // --- Text Search ---

    let mut results: Vec<Uuid> = vec![];
    let mut stalls: Vec<Uuid> = vec![];

    let query = format!(
        "
        SELECT
        id,
        stall_id
        FROM
        kueater.menuitem mi
        JOIN kueater.stall_menu sm ON mi.id = sm.menu_id
        WHERE name ILIKE '%{}%'
        LIMIT 200
        ", data.query
    );

    match sqlx::query_as::<_, (Uuid,Uuid)>(&query).fetch_all(pg_pool).await {
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"))
        }
        Ok(rows) => {
            let mut search_results = rows.iter().map(|(i,_)|*i).collect();
            let mut search_stalls = rows.iter().map(|(_,i)|*i).collect();
            results.append(&mut search_results);
            stalls.append(&mut search_stalls);
        }
    };

    // --- Vector Search ---

    let (tx, rx) = oneshot::channel::<String>();

    sender.send(AgentCommand { 
        msg: Command::Search { query: data.query.clone() },
        tx: Some(tx)
    }).await.unwrap();

    let mut vector_search_flag = true;
    let vectors = rx.await.map_err(|_| { vector_search_flag = false });

    if vector_search_flag {
        let query = format!(
            "
            SELECT
            object_id,
            stall_id
            FROM
            kueater.embeddings
            JOIN kueater.menuitem mi ON object_id = mi.id
            JOIN kueater.stall_menu sm ON mi.id = sm.menu_id
            WHERE object_type = 'menuitem'
            ORDER BY (1 - (embedding <=> '{}')) DESC
            LIMIT 200
            ", vectors.unwrap()
        );
        match sqlx::query_as::<_, (Uuid,Uuid)>(&query).fetch_all(pg_pool).await {
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Database failure"))
            }
            Ok(rows) => {
                let mut vector_results = rows.iter().map(|(i,_)|*i).collect();
                let mut vector_stalls = rows.iter().map(|(_,i)|*i).collect();
                results.append(&mut vector_results);
                stalls.append(&mut vector_stalls);
            }
        };
    }

    results.dedup();

    if results.len() <= 0 {
        return Ok(Response::new(SearchResponse { menus: vec![], stalls: vec![] }));
    }

    let conversions: Vec<MenuItem> = stream::iter(&results)
        .then(|uuid| async move {
            let item: MenuItem = sqlx::query_as(
                "SELECT * FROM kueater.get_menu_card_props($1, $2)"
            )
            .bind(&uuid)
            .bind(&user_id)
            .fetch_one(pg_pool).await.unwrap();
            return item
        }).collect().await;
    
    let stall_conversions: Vec<Stall> = stream::iter(&stalls_by_frequency(&stalls))
        .then(|uuid| async move {
            let stall: Stall = sqlx::query_as(
                "SELECT * FROM kueater.get_stall_data_props($1, $2)"
            )
            .bind(&uuid)
            .bind(&user_id)
            .fetch_one(pg_pool).await.unwrap();
            return stall
        }).collect().await;

        Ok(Response::new(
            SearchResponse {
                menus: conversions.iter().map(|i| types::MenuCardProps {
                    uuid: i.uuid.clone(),
                    name: i.name.clone(),
                    price: i.price,
                    likes: i.likes,
                    dislikes: i.dislikes,
                    stall_name: i.stall_name.clone(),
                    stall_lock: i.stall_lock.clone(),
                    image_url: i.image_url.clone(),
                    score: match i.score {
                        Some(v) => Some(v as f32),
                        None => None
                    },
                    reason: i.reason.clone(),
                    liked: i.liked,
                    disliked: i.disliked,
                    saved: i.saved
                }).collect(),
                stalls: stall_conversions.iter().map(|s| types::StallDataTypeProps {
                    uuid: s.uuid.clone(),
                    name: s.name.clone(),
                    rank: s.rank,
                    image_url: s.image_url.clone(),
                    location: s.location.clone(),
                    operating_hours: s.operating_hours.clone(),
                    price_range: s.price_range.clone(),
                    tags: s.tags.clone(),
                    reviews: s.reviews,
                    likes: s.likes,
                    rating: s.rating,
                    saved: s.saved
                }).collect()
                    }
                ))
}

fn stalls_by_frequency(input: &Vec<Uuid>) -> Vec<Uuid> {
    let mut freq_map: HashMap<Uuid, usize> = HashMap::new();
    for &id in input {
        *freq_map.entry(id).or_insert(0) += 1;
    }
    let mut freq_vec: Vec<(Uuid, usize)> = freq_map.into_iter().collect();
    freq_vec.sort_by(|a, b| {
        b.1.cmp(&a.1)
    });
    freq_vec.into_iter().map(|(id,_)|id).collect()
}