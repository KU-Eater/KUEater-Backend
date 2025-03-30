use num_traits::Zero;
use prost_types::Timestamp;
use sqlx::{types::{chrono::NaiveDateTime, Uuid}, PgPool, Row};
use tonic::{Request, Response, Status};

use super::kueater::{data::review::{ListReviewsRequest, ListReviewsResponse, PostReviewRequest, PostReviewResponse}, review::ContextItems, LocalizedString, MenuItem, Review, Stall, UserProfile};

fn sqlx_to_prost(ts: sqlx::types::chrono::NaiveDateTime) -> Timestamp {
    Timestamp {
        seconds: ts.and_utc().timestamp(),
        nanos: ts.and_utc().timestamp_subsec_nanos() as i32
    }
}

pub async fn list_reviews(
    pg_pool: &PgPool,
    request: Request<ListReviewsRequest>
) -> Result<Response<ListReviewsResponse>, Status> {
    // List review based on stall
    let data = request.into_inner();
    
    let mut query = format!(
        "SELECT
        kueater.review.id AS id,
        kueater.review.author AS author,
        kueater.review.content AS content,
        kueater.review.score AS score,
        kueater.review.created AS created,
        kueater.review.updated AS updated,
        kueater.menuitem.name AS context,
        kueater.stall.name AS stall
        FROM kueater.review
        JOIN kueater.review_context ON kueater.review.id = kueater.review_context.review_id
        JOIN kueater.menuitem ON kueater.review_context.menu_id = kueater.menuitem.id
        JOIN kueater.stall ON kueater.review.stall = kueater.stall.id
        WHERE kueater.review.stall = '{}'
        "
    , &data.stall);

    if !data.page_size.is_zero() {
        // Add to query
        query = format!("{} LIMIT {}", query, data.page_size);
    }

    let result = match sqlx::query(&query)
        .fetch_all(pg_pool)
        .await {
            Ok(rows) => rows,
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Internal error"))
            }
        };
    
    let mut reviews: Vec<Review> = vec![];
    for row in result {
        reviews.push(Review {
            uuid: String::from(row.get::<Uuid, &str>("id")),
            author: None,
            stall: Some(Stall {
                uuid: data.stall.clone(),
                name: Some( LocalizedString {content: row.get("stall"), locale: String::from("en") } ),
                lock: 0,
                items: vec![],
                image: String::from(""),
                dish_type: None
            }),
            context: Some(ContextItems {
                items: vec![MenuItem {
                    uuid: String::from(""),
                    name: Some(LocalizedString{content: row.get("context"), locale: String::from("en")}),
                    price: 0.0,
                    ingredients: vec![],
                    image: String::from(""),
                    tags: vec![]
                }]
            }),
            content: row.get("content"),
            created_at: Some(sqlx_to_prost(
                row.get::<NaiveDateTime, &str>("created")
            )),
            updated_at: Some(sqlx_to_prost(
                row.get::<NaiveDateTime, &str>("updated")
            )),
            rating: row.get("score")
        });
    }

    Ok(Response::new(ListReviewsResponse {
        reviews: reviews
    }))
}

pub async fn post_review(
    pg_pool: &PgPool,
    request: Request<PostReviewRequest>
) -> Result<Response<PostReviewResponse>, Status> {
    let data = request.into_inner();

    if data.author.is_empty() { return Err(Status::invalid_argument("No author")) }
    
    let prequery = format!("SELECT id FROM kueater.userprofile WHERE id = '{}'", data.author);
    let _ = match sqlx::query(&prequery).fetch_one(pg_pool).await {
        Err(_) => return Err(Status::not_found("User ID does not exist")),
        Ok(_) => ()
    };

    if data.content.is_empty() { return Err(Status::invalid_argument("No content message")) }
    if data.item_ids.is_empty() { return Err(Status::invalid_argument("Require at least one item as context") )}
    if 5 < data.rating && data.rating <= 0 {
        return Err(Status::invalid_argument("Review score can only be 1-5"))
    }

    let context = data.item_ids.first().expect("");

    let query = format!(
        "INSERT INTO
        kueater.review
        (author, stall, content, score, created, updated)
        VALUES
        ('{author}', '{stall}', E'{content}', {score}, {created}, {created})
        RETURNING ID
        "
    , author=data.author, stall=data.stall_id, content=data.content,
    score=data.rating, created="CURRENT_TIMESTAMP");

    let result = match sqlx::query(&query).fetch_one(pg_pool).await {
        Ok(res) => res,
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"))
        }
    };

    let relationship_query = format!(
        "INSERT INTO
        kueater.review_context
        (review_id, menu_id)
        VALUES
        ('{}', '{}')
        ", String::from(result.get::<Uuid, &str>("id")), context
    );

    let _ = match sqlx::query(&relationship_query).execute(pg_pool).await {
        Ok(res) => res,
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"))
        }
    };

    let resp = Review {
        uuid: String::from(result.get::<Uuid, &str>("id")),
        author: None,
        stall: None,
        context: None,
        content: String::from(""),
        created_at: None,
        updated_at: None,
        rating: data.rating
    };

    Ok(Response::new(PostReviewResponse{
        review: Some(resp)
    }))
}