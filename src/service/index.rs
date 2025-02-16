use sqlx::{types::{Uuid, Decimal}, PgPool, Row};
use tonic::{Request, Response, Status};
use num_traits::cast::ToPrimitive;

use crate::service::kueater::{LocalizedString, MenuItem};

use super::kueater::data::index::{
    GetMenuListingsRequest, GetMenuListingsResponse, CardedMenuItem,
    TopMenu, TopMenuRequest, SortStrategy
};

pub async fn get_menu_listing(
    pg_pool: &PgPool,
    request: Request<GetMenuListingsRequest>
) -> Result<Response<GetMenuListingsResponse>, Status> {

    println!("Running menu listings");

    // TODO: Use sort strategy,
    // For now, let's get menu listing sortable by UUID
    let data = request.into_inner();

    let reversed_sorted: bool = data.reversed_sort;
    let mut query = format!("SELECT * FROM {table}",
        table="kueater.menuitem"
    );

    if !data.page_token.is_empty() {
        query = format!("{} WHERE (id) {direction} ({token})", query,
            direction=(|| if reversed_sorted {
                "<"
            } else {
                ">"
            })(),
            token = data.page_token
        )
    }

    query = format!("{} ORDER BY id {direction} LIMIT 10", query,
        direction=(|| if reversed_sorted {
            "DESC"
        } else {
            "ASC"
        })()
    );

    let result = match sqlx::query(&query)
        .fetch_all(pg_pool)
        .await {
            Ok(rows) => rows,
            Err(_) => return Err(Status::resource_exhausted("No more results found for the menu listing"))
        };
    
    let mut items: Vec<CardedMenuItem> = vec![];
    for row in result {
        items.push(CardedMenuItem {
            item: Some(MenuItem {
                uuid: String::from(row.get::<Uuid, &str>("id")),
                name: Some(LocalizedString {
                    content: row.get("name"),
                    locale: String::from("en")
                }),
                price: row.get::<Decimal, &str>("price").to_f64().expect("Cannot parse price"),
                ingredients: vec![],
                image: String::from(""),
                tags: vec![]
            }),
            stall_name: Some(LocalizedString { content: String::from("Test"), locale: String::from("en") }),
            stall_lock: 1,
            likes: 1,
            liked_by_user: false,
            disliked_by_user: false,
            favorite_by_user: false
        });
    }

    let data = GetMenuListingsResponse {
        items: items,
        next_page_token: String::from("")
    };

    Ok(Response::new(data))
}

pub async fn index_top_menu(
    request: Request<TopMenuRequest>
) -> Result<Response<TopMenu>, Status> {

    println!("Running top menu");

    let data = TopMenu {
        items: vec![]
    };

    Ok(Response::new(data))
}