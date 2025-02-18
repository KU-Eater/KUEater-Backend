use sqlx::{query, types::{Decimal, Uuid}, PgPool, Row};
use tonic::{Request, Response, Status};
use num_traits::{cast::ToPrimitive, Zero};

use super::kueater::{data::{
    GetMenuRequest, GetMenuResponse,
    GetStallRequest, GetStallResponse,
}, Ingredient, LocalizedString, MenuItem, Stall};

struct SQLMenuItem {
    id: String,
    name: LocalizedString,
    price: f64,
    ingredients: Vec<Ingredient>,
    image: String
}

struct SQLStall {
    id: String,
    name: LocalizedString,
    lock: i32,
    image: String,
    dish_type: LocalizedString
}

pub async fn get_menu_item(
    pg_pool: &PgPool,
    request: Request<GetMenuRequest>
) -> Result<Response<GetMenuResponse>, Status> {

    let data = request.into_inner();

    if data.uuid.is_empty() { return Err(Status::invalid_argument("No UUID specified")) }

    let query = format!("SELECT 
    kueater.menuitem.id AS id,
    kueater.menuitem.name AS name,
    price,
    array_agg(kueater.ingredient.name) AS ingredients,
    kueater.menuitem.image AS image \
    FROM kueater.menuitem
    LEFT JOIN kueater.menu_ingredient ON kueater.menuitem.id = kueater.menu_ingredient.menu_id
    LEFT JOIN kueater.ingredient ON kueater.menu_ingredient.ingredient_id = kueater.ingredient.id
    WHERE kueater.menuitem.id = '{}'
    GROUP BY kueater.menuitem.id, kueater.menuitem.name, price, kueater.menuitem.image", data.uuid);

    let result: SQLMenuItem = match sqlx::query(&query)
        .fetch_one(pg_pool)
        .await {
            Err(e) => {
                println!("{}", e);
                return Err(Status::not_found("Menu item not found"))
            }
            Ok(item) => SQLMenuItem {
                id: String::from(item.get::<Uuid, &str>("id")),
                name: LocalizedString { content: item.get("name"), locale: String::from("en") },
                price: item.get::<Decimal, &str>("price").to_f64().expect("Cannot parse price"),
                ingredients: {
                    let ingredients_queried: Vec<String> = match item.try_get("ingredients") {
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
                image: item.get("image")
            }
        };
    
    let resp: GetMenuResponse = GetMenuResponse {
        item: Some(MenuItem {
            uuid: result.id,
            name: Some(result.name),
            price: result.price,
            ingredients: result.ingredients.into(),
            image: result.image,
            tags: vec![]
        })
    };

    Ok(Response::new(resp))
}

pub async fn get_stall(
    pg_pool: &PgPool,
    request: Request<GetStallRequest>
) -> Result<Response<GetStallResponse>, Status> {

    let data = request.into_inner();
    if data.uuid.is_empty() { return Err(Status::invalid_argument("No UUID specified")) }

    let query = format!("SELECT
        kueater.stall.id AS id,
        kueater.stall.name AS name,
        kueater.stall.lock AS lock,
        kueater.stall.image AS image,
        kueater.stall.dish_type AS dish_type \
        FROM kueater.stall
        WHERE kueater.stall.id = '{}'
    ", data.uuid);

    let result: SQLStall = match sqlx::query(&query).fetch_one(pg_pool).await {
        Err(e) => {
            println!("{}", e);
            return Err(Status::not_found("Stall not found"))
        }
        Ok(item) => SQLStall {
            id: String::from(item.get::<Uuid, &str>("id")),
            name: LocalizedString { content: item.get("name"), locale: String::from("en") },
            lock: item.get("lock"),
            image: item.get("image"),
            dish_type: LocalizedString { content: item.get("dish_type"), locale: String::from("en") }
        }
    };

    Ok(Response::new(GetStallResponse {
        stall: Some(
            Stall {
                uuid: result.id,
                name: Some(result.name),
                lock: result.lock,
                items: vec![],
                image: result.image,
                dish_type: Some(result.dish_type)
            }
        )
    }))
}