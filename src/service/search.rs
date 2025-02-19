use sqlx::{PgPool, Row, types::{Uuid, Decimal}};
use tonic::{Request, Response, Status};
use num_traits::ToPrimitive;

use crate::service::kueater::{MenuItem,LocalizedString,};

use super::kueater::data::search::{
    SearchRequest, SearchResponse, CardedMenuItem, CardedStall, SortStrategy,
    search_response::SearchResult, search_response::search_result::Result::{Item, Stall}
};

pub async fn search(
    pg_pool: &PgPool,
    request: Request<SearchRequest>
) -> Result<Response<SearchResponse>, Status> {

    let data = request.into_inner();

    if data.query.is_empty() { return Err(Status::invalid_argument("Search query cannot be empty")) }

    // TODO: Use vectors and embeddings to power search instead.

    let menu_query = format!(
        "SELECT
        kueater.menuitem.id AS id,
        kueater.menuitem.name AS name,
        price,
        kueater.menuitem.image AS image,
        kueater.stall.name AS stall_name,
        lock
        FROM kueater.menuitem
        JOIN kueater.stall_menu ON kueater.stall_menu.menu_id = kueater.menuitem.id
        JOIN kueater.stall ON kueater.stall.id = kueater.stall_menu.stall_id
        WHERE kueater.menuitem.name LIKE '%{}%'
        "
    , data.query);

    let menus = match sqlx::query(&menu_query).fetch_all(pg_pool).await {
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"))
        }
        Ok(v) => v
    };

    // TODO: Stall querying

    let mut results: Vec<SearchResult> = vec![];
    for row in menus {
        results.push(
            SearchResult { result: Some(
                Item(
                    CardedMenuItem {
                        item: Some(MenuItem {
                            uuid: String::from(row.get::<Uuid, &str>("id")),
                            name: Some(LocalizedString {
                                content: row.get("name"),
                                locale: String::from("en")
                            }),
                            price: row.get::<Decimal, &str>("price").to_f64().expect("Cannot parse price"),
                            ingredients: vec![],
                            image: row.get("image"),
                            tags: vec![]
                        }),
                        stall_name: Some(LocalizedString { 
                            content: row.get("stall_name"), locale: String::from("en") 
                        }),
                        stall_lock: row.get("lock"),
                        likes: 1,
                
                        // TODO: Respect user profile
                        liked_by_user: false,
                        disliked_by_user: false,
                        favorite_by_user: false
                    }
                )
            ) }
        );
    }

    let resp = SearchResponse {
        results: results,
        next_page_token: String::from("")
    };

    Ok(Response::new(resp))
}