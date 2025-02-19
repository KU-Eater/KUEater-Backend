use sqlx::{postgres::PgRow, types::{Decimal, Uuid}, PgPool, Row};
use tonic::{Request, Response, Status};
use num_traits::{cast::ToPrimitive, Zero};

use crate::service::kueater::{Ingredient, LocalizedString, MenuItem};

use super::kueater::{data::index::{
    top_menu::RankedCardedMenuItem, CardedMenuItem, CardedStall, GetMenuListingsRequest, GetMenuListingsResponse, SortStrategy, TopMenu, TopMenuRequest, TopStall, TopStallRequest
}, Stall};
use super::kueater::data::index::top_stall::RankedCardedStallItem;

const DEFAULT_PAGE_SIZE: i32 = 12;

pub async fn get_menu_listing(
    pg_pool: &PgPool,
    request: Request<GetMenuListingsRequest>
) -> Result<Response<GetMenuListingsResponse>, Status> {

    // TODO: Use sort strategy,
    // For now, let's get menu listing sortable by UUID
    let data = request.into_inner();
    let mut started_where: bool = false;

    let reversed_sorted: bool = data.reversed_sort;
    let mut query = format!("SELECT 
    kueater.menuitem.id AS id,
    kueater.menuitem.name AS name,
    price,
    array_agg(kueater.ingredient.name) AS ingredients,
    kueater.menuitem.image AS image,
    kueater.stall.name AS stall_name,
    lock \
    FROM kueater.menuitem
    LEFT JOIN kueater.menu_ingredient ON kueater.menuitem.id = kueater.menu_ingredient.menu_id
    LEFT JOIN kueater.ingredient ON kueater.menu_ingredient.ingredient_id = kueater.ingredient.id
    JOIN kueater.stall_menu ON kueater.stall_menu.menu_id = kueater.menuitem.id
    JOIN kueater.stall ON kueater.stall.id = kueater.stall_menu.stall_id");

    if !data.page_token.is_empty() {
        query = format!("{}
        WHERE (kueater.menuitem.id) {direction} ('{token}')", query,
            direction=(|| if reversed_sorted {
                "<"
            } else {
                ">"
            })(),
            token = data.page_token
        );
        started_where = true;
    }

    if !data.match_lock.is_zero() {
        if !started_where {
            query = format!("{}
            WHERE kueater.stall.lock = {}
            ", query, data.match_lock);
            started_where = true;
        } else {
            query = format!("{} AND kueater.stall.lock = {}
            ", query, data.match_lock)
        }
    }

    query = format!("{}
    GROUP BY kueater.menuitem.id, kueater.menuitem.name, price, kueater.menuitem.image, kueater.stall.name, lock
    ORDER BY kueater.menuitem.id {direction} LIMIT {limit}", query,
        direction=(|| if reversed_sorted {
            "DESC"
        } else {
            "ASC"
        })(),
        limit=(|| if data.page_size.is_zero() {
            DEFAULT_PAGE_SIZE
        } else {
            data.page_size
        })()
    );

    let result = match sqlx::query(&query)
        .fetch_all(pg_pool)
        .await {
            Ok(rows) => rows,
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Internal error"))
            }
        };
    
    let mut items: Vec<CardedMenuItem> = vec![];
    for row in &result {
        items.push(
            CardedMenuItem {
                item: Some(MenuItem {
                    uuid: String::from(row.get::<Uuid, &str>("id")),
                    name: Some(LocalizedString {
                        content: row.get("name"),
                        locale: String::from("en")
                    }),
                    price: row.get::<Decimal, &str>("price").to_f64().expect("Cannot parse price"),
                    ingredients: {
                        let ingredients_queried: Vec<String> = match row.try_get("ingredients") {
                            Ok(v) => v,
                            Err(_) => vec![]
                        };
                        let mut ingredients: Vec<Ingredient> = vec![];
                        for i in ingredients_queried {
                            ingredients.push(Ingredient {
                                uuid: String::from(""),
                                name: Some(LocalizedString {content: i, locale: String::from("en")})
                            })
                        };
                        ingredients
                    },
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
        );
    }

    if items.is_empty() {
        // If there's no result, just return end of page
        return Err(Status::resource_exhausted("End of page"))
    }

    let data = GetMenuListingsResponse {
        items: items,
        next_page_token: String::from(result.last().expect("Expected free token ID").get::<Uuid, &str>("id"))
    };

    Ok(Response::new(data))
}

pub async fn index_top_menu(
    pg_pool: &PgPool,
    _request: Request<TopMenuRequest>
) -> Result<Response<TopMenu>, Status> {
    let query = format!("SELECT
        kueater.menuitem.id AS id,
        kueater.menuitem.name AS name,
        price,
        kueater.menuitem.image AS image,
        kueater.stall.name AS stall_name,
        lock
        FROM kueater.menuitem
        JOIN kueater.stall_menu ON kueater.stall_menu.menu_id = kueater.menuitem.id
        JOIN kueater.stall ON kueater.stall.id = kueater.stall_menu.stall_id
        LIMIT 3
    ");

    let result = match sqlx::query(&query).fetch_all(pg_pool).await {
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"))
        }
        Ok(v) => v
    };

    let mut cards: Vec<RankedCardedMenuItem> = vec![];
    for (i, row) in result.iter().enumerate() {
        cards.push(
            RankedCardedMenuItem { 
                item: Some(CardedMenuItem {
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
                }),
                rank: i.to_i32().expect("Cannot convert to i32") + 1
            }
        )
    }

    Ok(Response::new(TopMenu {
        items: cards
    }))
}

pub async fn index_top_stall(
    pg_pool: &PgPool,
    _request: Request<TopStallRequest>
) -> Result<Response<TopStall>, Status> {
    let query = format!("SELECT
        kueater.stall.id AS id,
        kueater.stall.name AS name,
        kueater.stall.lock AS lock,
        kueater.stall.image AS image,
        kueater.stall.dish_type AS dish_type,
        MIN(kueater.menuitem.price) AS min_price,
        MAX(kueater.menuitem.price) AS max_price
        FROM kueater.stall
        JOIN kueater.stall_menu ON kueater.stall_menu.stall_id = kueater.stall.id
        JOIN kueater.menuitem ON kueater.menuitem.id = kueater.stall_menu.menu_id
        GROUP BY kueater.stall.id, kueater.stall.name, kueater.stall.lock,
        kueater.stall.image, kueater.stall.dish_type
        LIMIT 3
    ");
    
    let result = match sqlx::query(&query).fetch_all(pg_pool).await {
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"))
        }
        Ok(v) => v
    };

    let mut cards: Vec<RankedCardedStallItem> = vec![];
    for (i, row) in result.iter().enumerate() {
        cards.push(
            RankedCardedStallItem {
                stall: Some(CardedStall {
                    stall: Some(Stall {
                        uuid: String::from(row.get::<Uuid, &str>("id")),
                        name: Some(LocalizedString { content: row.get("name"), locale: String::from("en") }),
                        lock: row.get("lock"),
                        image: row.get("image"),
                        items: vec![],
                        dish_type: Some(LocalizedString { content: row.get("dish_type"), locale: String::from("en") })
                    }),
                    review_count: 0,
                    like_count: 0,
                    rating: 5.0,
                    min_price: row.get::<Decimal, &str>("min_price").to_i32().expect("Cannot parse price"),
                    max_price: row.get::<Decimal, &str>("max_price").to_i32().expect("Cannot parse price"),
                    favorite_by_user: false
                }),
                rank: i.to_i32().expect("Cannot convert to i32") + 1
            }
        );
    }

    Ok(Response::new(
        TopStall {
            stalls: cards
        }
    ))
}