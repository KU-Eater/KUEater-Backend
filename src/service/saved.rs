use crate::no_impl;

use super::backend::{Send, Recv};
use super::kueater::data::types;
use super::kueater::{Empty, data::*};

use futures::{stream, StreamExt};
use serde::Deserialize;
use sqlx::types::{Decimal, Uuid};
use sqlx::{Error, PgPool};
use tonic::{Response, Status};

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

pub async fn saved_items(
    pg_pool: &PgPool,
    request: Recv<SavedItemsRequest>
) -> Send<SavedItemsResponse> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let mut items_to_query: Vec<Uuid> = vec![];

    match sqlx::query_as::<_, (Uuid,)>(
        "
        SELECT menu_id FROM kueater.saved_item WHERE user_id = $1
        "
    ).bind(user_id).fetch_all(pg_pool).await {
        Ok(rows) => {
            items_to_query = rows.iter().map(|(i,)|*i).collect();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    let conversions: Vec<MenuItem> = stream::iter(&items_to_query)
        .then(|uuid| async move {
            let item: MenuItem = sqlx::query_as(
                "SELECT * FROM kueater.get_menu_card_props($1, $2)"
            )
            .bind(&uuid)
            .bind(&user_id)
            .fetch_one(pg_pool).await.unwrap();
            return item
        }).collect().await;

    Ok(Response::new(
        SavedItemsResponse {
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
        }).collect()
    }
    ))
}

pub async fn saved_stalls(
    pg_pool: &PgPool,
    request: Recv<SavedStallsRequest>
) -> Send<SavedStallsResponse> {

    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let mut items_to_query: Vec<Uuid> = vec![];

    match sqlx::query_as::<_, (Uuid,)>(
        "
        SELECT stall_id FROM kueater.saved_stall WHERE user_id = $1
        "
    ).bind(user_id).fetch_all(pg_pool).await {
        Ok(rows) => {
            items_to_query = rows.iter().map(|(i,)|*i).collect();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    let stall_conversions: Vec<Stall> = stream::iter(&items_to_query)
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
        SavedStallsResponse {
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