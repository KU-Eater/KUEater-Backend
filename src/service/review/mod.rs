use serde::Deserialize;
use sqlx::types::Uuid;
use sqlx::{Error, PgPool, Row};
use tonic::{Response, Status};


use super::backend::{Send, Recv};
use super::kueater::data::types;
use super::kueater::data::review::*;

#[derive(Debug, Deserialize, sqlx::FromRow)]
struct Review {
    uuid: String,
    username: String,
    role: String,
    gender: String,
    created_at: String,
    stars: i32,
    content: String
}

pub async fn list_reviews(
    pg_pool: &PgPool,
    request: Recv<ListReviewsRequest>
) -> Send<ListReviewsResponse> {
    // List reviews using stall id

    let data = request.into_inner();

    let stall_id = match data.stall_id.parse::<Uuid>() {
        Ok(res) => res,
        Err(_) => {
            return Err(Status::invalid_argument("Stall id not a UUID"));
        }
    };

    let amount = data.page_size;

    // Query
    let query = format!(
        "
        SELECT
        r.id::TEXT AS uuid,
        up.name AS username,
        up.role::TEXT AS role,
        up.gender::TEXT AS gender,
        r.created::TEXT AS created_at,
        r.score AS stars,
        r.content AS content
        FROM kueater.review r
        JOIN kueater.userprofile up ON author = up.id
        WHERE stall = $1
        ORDER BY created DESC
        LIMIT $2
        "
    );

    let reviews: Result<Vec<Review>, Error> = sqlx::query_as(&query)
        .bind(stall_id).bind(amount).fetch_all(pg_pool).await;

    let review_vec = match reviews {
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Cannot get stall item"));
        }
        Ok(rows) => {
            rows
        }
    };

    // Cannot make a struct so rating summary are going to be a bit traditional
    let summary_res = match sqlx::query(
        "SELECT * FROM kueater.get_stall_rating_summary($1)"
    ).bind(stall_id).fetch_one(pg_pool).await {
        Ok(row) => row,
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database error"))
        }
    };

    let summary = RatingSummary {
        avg_stall_rating: summary_res.get::<f64, &str>("avg_stall_rating"),
        total_reviews: summary_res.get::<i32, &str>("total_reviews"),
        total_likes: summary_res.get::<i32, &str>("total_likes"),
        total_menu_saved: summary_res.get::<i32, &str>("total_menu_saved"),
        total_stall_saved: summary_res.get::<i32, &str>("total_stall_saved"),
        by_stars: Some(ByStarGroup {
            one: Some(StarRating { total: summary_res.get::<i32, &str>("one_star_total"), percent: summary_res.get::<f64, &str>("one_star_percent") }),
            two: Some(StarRating { total: summary_res.get::<i32, &str>("two_star_total"), percent: summary_res.get::<f64, &str>("two_star_percent") }),
            three: Some(StarRating { total: summary_res.get::<i32, &str>("three_star_total"), percent: summary_res.get::<f64, &str>("three_star_percent") }),
            four: Some(StarRating { total: summary_res.get::<i32, &str>("four_star_total"), percent: summary_res.get::<f64, &str>("four_star_percent") }),
            five: Some(StarRating { total: summary_res.get::<i32, &str>("five_star_total"), percent: summary_res.get::<f64, &str>("five_star_percent") })
        })
    };

    let stall_name = match sqlx::query(
        &format!("SELECT id, name FROM kueater.stall WHERE id = '{}'", stall_id)
    ).fetch_one(pg_pool).await {
        Ok(row) => row.get::<String, &str>("name"),
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Database error"))
        }
    };

    Ok(Response::new(
        ListReviewsResponse {
            stall_id: stall_id.to_string(),
            stall_name: stall_name,
            rating_summary: Some(summary),
            reviews: review_vec.iter().map(|r| types::ReviewCardProps {
                uuid: r.uuid.clone(),
                username: r.username.clone(),
                role: r.role.clone(),
                gender: r.gender.clone(),
                created_at: r.created_at.clone(),
                stars: r.stars,
                content: r.content.clone()
            }).collect()
        }
    ))

}

pub async fn post_review(
    pg_pool: &PgPool,
    request: Recv<PostReviewRequest>
) -> Send<PostReviewResponse> {

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

    let review_id = match sqlx::query(
        "
        INSERT INTO kueater.review (author, stall, score, content) VALUES
        ($1, $2, $3, $4) RETURNING id
        "
    ).bind(&user_id).bind(&stall_id).bind(data.rating).bind(&data.content.clone())
    .fetch_one(pg_pool).await {
        Ok(row) => row.get::<Uuid, &str>("id"),
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"))
        }
    };

    let query = format!(
        "
        SELECT
        r.id::TEXT AS uuid,
        up.name AS username,
        up.role::TEXT AS role,
        up.gender::TEXT AS gender,
        r.created::TEXT AS created_at,
        r.score AS stars,
        r.content AS content
        FROM kueater.review r
        JOIN kueater.userprofile up ON author = up.id
        WHERE r.id = $1
        ORDER BY created DESC
        "
    );

    let review: Review = sqlx::query_as(&query)
        .bind(review_id).fetch_one(pg_pool).await.unwrap();

    Ok(Response::new(
        PostReviewResponse { review: Some(
            types::ReviewCardProps { 
                uuid: review.uuid, 
                username: review.username, 
                role: review.role, 
                gender: review.gender, 
                created_at: review.created_at, 
                stars: review.stars, 
                content: review.content
            }
        ) }
    ))

}