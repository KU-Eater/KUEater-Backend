use futures::{stream, StreamExt};
use serde::Deserialize;
use sqlx::types::Uuid;
use sqlx::{Error, PgPool};
use tonic::{Response, Status};

use super::backend::{Send, Recv};
use super::kueater::{Empty, data::*};

#[derive(Debug, Deserialize, sqlx::FromRow)]
struct Prefereces {
    #[sqlx(rename = "id")]
    user_id: Uuid,
    email: String,
    #[sqlx(rename = "name")]
    username: String,
    gender: String,
    role: String,
    #[sqlx(rename = "diets")]
    dietary: Vec<String>,
    allergies: Vec<String>,
    cuisines: Vec<String>,
    #[sqlx(rename = "disliked_ingredients")]
    dislikes: Vec<String>,
    #[sqlx(rename = "favorite_dishes")]
    likes: Vec<String>
}

#[derive(Debug, Deserialize, sqlx::FromRow)]
struct MenuItem {
    uuid: String,
    name: String,
    price: f64,
    likes: i32,
    dislikes: i32,
    stall_id: String,
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

pub async fn get_preferences(
    pg_pool: &PgPool,
    request: Recv<GetPreferencesRequest>
) -> Send<GetPreferencesResponse> {

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

    let pref: Result<Prefereces, Error> = sqlx::query_as(
        "
        SELECT u.id, email, name, gender::text, role::text, diets::text[], allergies::text[], cuisines, disliked_ingredients, favorite_dishes FROM
        kueater.userprofile u
        JOIN kueater.user_profile_preferences upf ON u.id = upf.user_id
        JOIN kueater.user_preferences up ON upf.preferences_id = up.id
        WHERE u.id = $1
        "
    ).bind(&user_id).fetch_one(pg_pool).await;

    match pref {
        Ok(p) => {
            Ok(Response::new(
                GetPreferencesResponse {
                    user_id: p.user_id.to_string(),
                    email: p.email,
                    username: p.username,
                    gender: p.gender,
                    role: p.role,
                    dietary: p.dietary,
                    allergies: p.allergies,
                    cuisines: p.cuisines,
                    dislikes: p.dislikes,
                    likes: p.likes
                }
            ))
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Cannot get preferences"));
        }
    }
}

pub async fn get_menu_item(
    pg_pool: &PgPool,
    request: Recv<GetMenuItemRequest>
) -> Send<types::MenuCardProps> {
    let extensions = request.extensions().clone();
    let data = request.into_inner();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let menu_id = match data.item_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Menu id not a UUID"));
        }
    };

    let item: Result<MenuItem, Error> = sqlx::query_as(
        "SELECT * FROM kueater.get_menu_card_props($1, $2)"
    )
    .bind(&menu_id)
    .bind(&user_id)
    .fetch_one(pg_pool).await;

    match item {
        Ok(i) => {
            Ok(Response::new(
                types::MenuCardProps {
                    uuid: i.uuid,
                    name: i.name,
                    price: i.price,
                    likes: i.likes,
                    dislikes: i.dislikes,
                    stall_id: i.stall_id,
                    stall_name: i.stall_name,
                    stall_lock: i.stall_lock,
                    image_url: i.image_url,
                    score: match i.score {
                        Some(v) => Some(v as f32),
                        None => None
                    },
                    reason: i.reason,
                    liked: i.liked,
                    disliked: i.disliked,
                    saved: i.saved
                }
            ))
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Cannot get menu item"));
        }
    }
}

pub async fn get_stall(
    pg_pool: &PgPool,
    request: Recv<GetStallRequest>
) -> Send<types::StallDataTypeProps> {
    let extensions = request.extensions().clone();
    let data = request.into_inner();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let stall_id = match data.stall_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Stall id not a UUID"));
        }
    };

    let stall: Result<Stall, Error> = sqlx::query_as(
        "SELECT * FROM kueater.get_stall_data_props($1, $2)"
    )
    .bind(&stall_id)
    .bind(&user_id)
    .fetch_one(pg_pool).await;

    match stall {
        Ok(s) => {
            Ok(Response::new(
                types::StallDataTypeProps {
                    uuid: s.uuid,
                    name: s.name,
                    rank: s.rank,
                    image_url: s.image_url,
                    location: s.location,
                    operating_hours: s.operating_hours,
                    price_range: s.price_range,
                    tags: s.tags,
                    reviews: s.reviews,
                    likes: s.likes,
                    rating: s.rating,
                    saved: s.saved
                }
            ))
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Cannot get stall item"));
        }
    }
}

pub async fn items_in_stall(
    pg_pool: &PgPool,
    request: Recv<StallItemsRequest>
) -> Send<types::MenuCardGridConstructor> {

    let extensions = request.extensions().clone();
    let data = request.into_inner();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let stall_id = match data.stall_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Stall id not a UUID"));
        }
    };

    let mut items_to_query: Vec<Uuid> = vec![];

    match sqlx::query_as::<_, (Uuid,)>(
        "SELECT menu_id AS id FROM kueater.stall_menu WHERE stall_id = $1"
    ).bind(&stall_id).fetch_all(pg_pool).await {
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
        types::MenuCardGridConstructor {
            data: conversions.iter().map(|i| types::MenuCardProps {
                uuid: i.uuid.clone(),
                name: i.name.clone(),
                price: i.price,
                likes: i.likes,
                dislikes: i.dislikes,
                stall_id: i.stall_id.clone(),
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