use futures::{stream, StreamExt};
use serde::Deserialize;
use sqlx::types::{Decimal, Uuid};
use sqlx::{Error, PgPool};
use tonic::{Response, Status};

use crate::service::kueater::data::types::MenuCardHorizontalConstructor;

use super::backend::{Send, Recv};
use super::kueater::data::types;
use super::kueater::{Empty, data::home::*};

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

pub async fn top_menu(
    pg_pool: &PgPool,
    request: Recv<Empty>
) -> Send<TopMenuProps> {
    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let mut items_to_query: Vec<Uuid> = vec![];

    match sqlx::query_as::<_, (Uuid,i64)>(
        "
        SELECT id, COUNT(lk.user_id) as count FROM kueater.menuitem
        LEFT JOIN kueater.liked_item lk ON lk.menu_id = id
        GROUP BY id LIMIT 40
        "
    ).fetch_all(pg_pool).await {
        Ok(rows) => {
            items_to_query = rows.iter().map(|(i,_)|*i).collect();
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
            TopMenuProps { props: Some(
            types::MenuCardHorizontalConstructor {
                menus: conversions.iter().map(|i| types::MenuCardProps {
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
                }).collect(),
                title: Some("Top Menus of People".to_string())
            }
            ) }
        ))
}

pub async fn top_stall(
    pg_pool: &PgPool,
    request: Recv<Empty>
) -> Send<TopStallProps> {
    let extensions = request.extensions().clone();

    let user_id = extensions.get::<super::UserContext>().unwrap().user_id.clone();

    let user_id = match user_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("User id not a UUID"));
        }
    };

    let stalls: Result<Vec<Stall>, Error> = sqlx::query_as(
        "SELECT * FROM kueater.multi_stall_data_props($1, $2)"
    ).bind(&user_id).bind(10).fetch_all(pg_pool).await;

    match stalls {
        Ok(stall_vec) => {
            Ok(Response::new(
                TopStallProps {
                    props: Some(
                         types::StallCardListConstructor {
                            data: stall_vec.iter().map(|s| types::StallDataTypeProps {
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
                    )
                }
            ))
        }
        Err(e) => {
            println!("{}", e);
            Err(Status::internal("Cannot get top stalls"))
        }
    }
}

async fn has_recommendations(
    pg_pool: &PgPool,
    user_id: &Uuid
) -> Result<bool, Status> {
    match sqlx::query_as::<_, (i64, )>(
        "
        SELECT COUNT(menu_id) FROM kueater.current_menuitem_scores
        WHERE user_id = $1
        "
    ).bind(user_id).fetch_one(pg_pool).await {
        Ok((v,)) if v > 0 => Ok(true),
        Ok(_) => Ok(false),
        Err(e) => {
            println!("{}", e);
            Err(Status::internal("Database error while checking"))
        }
    }
}

pub async fn infer_like(
    pg_pool: &PgPool,
    request: Recv<InferLikeMsg>
) -> Send<InferLikeProps> {

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

    match has_recommendations(pg_pool, &user_id).await {
        Ok(b) => {
            if !b {
                return Ok(Response::new(InferLikeProps {
                    props: Some(MenuCardHorizontalConstructor {
                        menus: vec![],
                        title: None
                    })
                }));
            }
        },
        Err(e) => {
            return Err(e);
        }
    }

    // Has recommendations
    let common_word = data.word.clone();

    let mut items_to_query: Vec<Uuid> = vec![];

    let __query = format!(
        "
        SELECT menu_id FROM kueater.current_menuitem_scores
        WHERE user_id = $1 AND LENGTH(reasoning) > 0 AND reasoning ILIKE '%{}%'
        ORDER BY RANDOM() LIMIT 10
        ", common_word
    );
  
    match sqlx::query_as::<_, (Uuid,)>(
        &__query
    ).bind(&user_id).fetch_all(pg_pool).await {
        Ok(rows) => {
            items_to_query = rows.iter().map(|(i,)|*i).collect();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    if items_to_query.len() <= 0 {
        return Ok(Response::new(InferLikeProps {
            props: Some(MenuCardHorizontalConstructor {
                menus: vec![],
                title: None
            })
        }));
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
        InferLikeProps {
            props: Some(
                types::MenuCardHorizontalConstructor {
                    menus: conversions.iter().map(|i| types::MenuCardProps {
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
                    }).collect(),
                    title: Some(format!("Because You Like {}", common_word))
                }
            )
        }
    ))
}

pub async fn for_you(
    pg_pool: &PgPool,
    request: Recv<ForYouMsg>
) -> Send<ForYouProps> {

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

    match has_recommendations(pg_pool, &user_id).await {
        Ok(b) => {
            if !b {
                return Ok(Response::new(ForYouProps {
                    props: Some(MenuCardHorizontalConstructor {
                        menus: vec![],
                        title: None
                    })
                }));
            }
        },
        Err(e) => {
            return Err(e);
        }
    }

    // Has recommendations
    let mut items_to_query: Vec<Uuid> = vec![];

    let __query = format!(
        "
        SELECT menu_id FROM kueater.current_menuitem_scores
        WHERE user_id = $1 AND score > 10 AND NOT
        (reasoning ILIKE '%diet%' OR reasoning ILIKE '%allergen%' OR reasoning ILIKE '%traces%')
        ORDER BY RANDOM() LIMIT 8
        "
    );
  
    match sqlx::query_as::<_, (Uuid,)>(
        &__query
    ).bind(&user_id).fetch_all(pg_pool).await {
        Ok(rows) => {
            items_to_query = rows.iter().map(|(i,)|*i).collect();
        }
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database failure"));
        }
    }

    if items_to_query.len() <= 0 {
        return Ok(Response::new(ForYouProps {
            props: Some(MenuCardHorizontalConstructor {
                menus: vec![],
                title: None
            })
        }));
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
        ForYouProps {
            props: Some(
                types::MenuCardHorizontalConstructor {
                    menus: conversions.iter().map(|i| types::MenuCardProps {
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
                    }).collect(),
                    title: Some(format!("For You"))
                }
            )
        }
    ))

}

enum TokenType {
    Default,
    ByRecommendation(i64),
    ByDatabaseIndex(Uuid)
}

fn check_index_token(
    token: &str
) -> Result<TokenType, Status> {
    if token.trim().is_empty() {
        return Ok(TokenType::Default);
    }

    if let Ok(t) = token.parse::<i64>() {
        return Ok(TokenType::ByRecommendation(t));
    }

    if let Ok(t) = token.parse::<Uuid>() {
        return Ok(TokenType::ByDatabaseIndex(t));
    }

    Err(Status::invalid_argument("Invalid index token"))
}

pub async fn get_recommendations(
    pg_pool: &PgPool,
    request: Recv<GetRecommendationsMsg>
) -> Send<RecommendationsList> {

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

    let index_token: i64 = match check_index_token(&data.index_token) {
        Ok(TokenType::Default) => {
            0
        },
        Ok(TokenType::ByRecommendation(v)) => {
            v
        },
        Ok(TokenType::ByDatabaseIndex(v)) => {
            // Let another function handle by database index
            return get_menu_page_by_db_index(
                pg_pool, &user_id, Some(v)).await;
        },
        Err(e) => {
            return Err(e.into());
        }
    };

    match has_recommendations(pg_pool, &user_id).await {
        Ok(b) => {
            if !b {
                return get_menu_page_by_db_index(
                    pg_pool, &user_id, None
                ).await;
            }
        },
        Err(e) => {
            return Err(e);
        }
    }

    let mut items_to_query: Vec<Uuid> = vec![];
    let mut next_index_token: String = String::new();
    let mut next_score_token: String = String::new();

    if index_token > 0 {
        // has index already
        // (score < k OR (score = k AND id > idx))
        let score_token = match data.score_token.parse::<Decimal>() {
            Ok(v) => v,
            Err(_) => {
                return Err(Status::invalid_argument("Score token is invalid"))
            }
        };

        match sqlx::query_as::<_, (i32, Uuid, Decimal)>(
            "
            SELECT id, menu_id, score FROM kueater.current_menuitem_scores
            WHERE user_id = $1 AND score > 5 AND NOT
            (reasoning ILIKE '%diet%' OR reasoning ILIKE '%allergen%' OR reasoning ILIKE '%traces%')
            AND (score < $2 OR (score < $2 AND id > $3))
            ORDER BY score DESC LIMIT 100
            "
        ).bind(&user_id).bind(score_token).bind(index_token).fetch_all(pg_pool)
        .await {
            Ok(rows) => {
                if rows.len() <= 0 {
                    return Err(Status::resource_exhausted("End of page"))
                }
                items_to_query = rows.iter().map(|(_,i,_)|*i).collect();
                let tokens = rows.last().clone().unwrap();
                next_index_token = tokens.0.to_string();
                next_score_token = tokens.2.to_string();
            }
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Database failure"));
            }
        }
    } else {
        // no index, start from beginning
        match sqlx::query_as::<_, (i32, Uuid, Decimal)>(
            "
            SELECT id, menu_id, score FROM kueater.current_menuitem_scores
            WHERE user_id = $1 AND score > 5 AND NOT
            (reasoning ILIKE '%diet%' OR reasoning ILIKE '%allergen%' OR reasoning ILIKE '%traces%')
            ORDER BY score DESC LIMIT 100
            "
        ).bind(&user_id).fetch_all(pg_pool)
        .await {
            Ok(rows) => {
                if rows.len() <= 0 {
                    return Err(Status::resource_exhausted("End of page"))
                }
                items_to_query = rows.iter().map(|(_,i,_)|*i).collect();
                let tokens = rows.last().clone().unwrap();
                next_index_token = tokens.0.to_string();
                next_score_token = tokens.2.to_string();
            }
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Database failure"));
            }
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
        RecommendationsList { 
            menu: conversions.iter().map(|i| types::MenuCardProps {
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
            }).collect(),
            next_index_token: next_index_token,
            score_token: next_score_token
        }
    ))

}

async fn get_menu_page_by_db_index(
    pg_pool: &PgPool,
    user_id: &Uuid,
    index_token: Option<Uuid>
) -> Send<RecommendationsList> {

    let mut items_to_query: Vec<Uuid> = vec![];
    let mut next_index_token: String = String::new();

    if index_token.is_some() {
        // Starting from index
        let token = index_token.unwrap();

        match sqlx::query_as::<_, (Uuid,)>(
            "
            SELECT id FROM kueater.menuitem WHERE id > $1 ORDER BY id LIMIT 100
            "
        ).bind(&token).fetch_all(pg_pool).await {
            Ok(rows) => {
                if rows.len() <= 0 {
                    return Err(Status::resource_exhausted("End of page"))
                }
                items_to_query = rows.iter().map(|(i,)|*i).collect();
                next_index_token = items_to_query.last().clone().unwrap().to_string();
            }
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Database failure"));
            }
        };

    } else {
        match sqlx::query_as::<_, (Uuid,)>(
            "
            SELECT id FROM kueater.menuitem ORDER BY id LIMIT 100
            "
        ).fetch_all(pg_pool).await {
            Ok(rows) => {
                if rows.len() <= 0 {
                    return Err(Status::resource_exhausted("End of page"))
                }
                items_to_query = rows.iter().map(|(i,)|*i).collect();
                next_index_token = items_to_query.last().clone().unwrap().to_string();
            }
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Database failure"));
            }
        };
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
        RecommendationsList { 
            menu: conversions.iter().map(|i| types::MenuCardProps {
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
            }).collect(),
            next_index_token: next_index_token,
            score_token: "".to_string()
        }
    ))
}