use sqlx::{types::{Decimal, Uuid}, PgPool, Row};
use tonic::{Request, Response, Status};
use tokio::sync::oneshot;
use num_traits::ToPrimitive;

use crate::{agent::command::{AgentCommand, Command}, service::kueater::{LocalizedString, MenuItem}, AgentCommandSender};

use super::kueater::data::search::{
    search_response::{search_result::Result::Item, SearchResult}, CardedMenuItem, SearchRequest, SearchResponse
};


struct SearchEntry {
    id: String,
    name: String,
    price: f64,
    image: String,
    stall_name: String,
    stall_lock: i32,
    likes: i32,
    liked_by_user: bool,
    disliked_by_user: bool,
    saved_by_user: bool
}

impl PartialEq for SearchEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub async fn search(
    pg_pool: &PgPool,
    sender: &AgentCommandSender,
    request: Request<SearchRequest>
) -> Result<Response<SearchResponse>, Status> {

    let data = request.into_inner();

    if data.query.is_empty() { return Err(Status::invalid_argument("Search query cannot be empty")) }

    let (tx, rx) = oneshot::channel::<String>();

    sender.send(AgentCommand {
        msg: Command::Search { query: data.query.clone() },
        tx: tx
    }).await.unwrap();

    let mut vector_search_flag = true;
    let mut search_results: Vec<SearchEntry> = vec![];
    let vectors = rx.await.map_err(|_| { vector_search_flag = false });

    if vector_search_flag {
        let query = format!(
            "SELECT
            object_id,
            1 - (embedding <=> '{}') AS similarity,
            mi.name AS name,
            price,
            mi.image AS image,
            st.name AS stall_name,
            lock,
            COUNT(likes.menu_id) AS likes,
            COALESCE(BOOL_OR(liked.user_id IS NOT NULL), FALSE) AS liked,
            COALESCE(BOOL_OR(disliked.user_id IS NOT NULL), FALSE) AS disliked,
            COALESCE(BOOL_OR(saved.user_id IS NOT NULL), FALSE) AS saved
            FROM kueater.embeddings e
            JOIN kueater.menuitem mi ON e.object_id = mi.id
            JOIN kueater.stall_menu stm ON stm.menu_id = mi.id
            JOIN kueater.stall st ON st.id = stm.stall_id
            LEFT JOIN kueater.liked_item likes ON likes.menu_id = mi.id
            LEFT JOIN kueater.liked_item liked ON 
                (liked.menu_id = mi.id AND liked.user_id = '{uid}')
            LEFT JOIN kueater.disliked_item disliked ON 
                (disliked.menu_id = mi.id AND disliked.user_id = '{uid}')
            LEFT JOIN kueater.saved_item saved ON 
                (saved.menu_id = mi.id AND saved.user_id = '{uid}')
            WHERE object_type = 'menuitem'
            GROUP BY 
                object_id, 
                similarity, 
                mi.name, 
                price, 
                mi.image, 
                st.name, 
                lock
            ORDER BY similarity DESC
            LIMIT 100
            "
        , vectors.unwrap(), uid=data.user);
        let results = match sqlx::query(&query).fetch_all(pg_pool).await {
            Err(e) => {
                println!("{}", e);
                return Err(Status::internal("Internal error"))
            }
            Ok(v) => v
        };
        for row in results {
            search_results.push(SearchEntry {
                id: row.get::<Uuid, &str>("object_id").to_string(),
                name: row.get("name"),
                price: row.get::<Decimal, &str>("price").to_f64().unwrap(),
                image: row.get("image"),
                stall_name: row.get("stall_name"),
                stall_lock: row.get("lock"),
                likes: row.get::<i64, &str>("likes").to_i32().unwrap(),
                liked_by_user: row.get::<bool, &str>("liked"),
                disliked_by_user: row.get::<bool, &str>("disliked"),
                saved_by_user: row.get::<bool, &str>("saved")
            });
        };
    }

    let menu_query = format!(
        "
        SELECT
        mi.id AS id,
        mi.name AS name,
        price,
        mi.image AS image,
        st.name AS stall_name,
        lock,
        COUNT(likes.menu_id) AS likes,
        COALESCE(BOOL_OR(liked.user_id IS NOT NULL), FALSE) AS liked,
        COALESCE(BOOL_OR(disliked.user_id IS NOT NULL), FALSE) AS disliked,
        COALESCE(BOOL_OR(saved.user_id IS NOT NULL), FALSE) AS saved
        FROM kueater.menuitem mi
        JOIN kueater.stall_menu stm ON stm.menu_id = mi.id
        JOIN kueater.stall st ON st.id = stm.stall_id
        LEFT JOIN kueater.liked_item likes ON likes.menu_id = mi.id
        LEFT JOIN kueater.liked_item liked ON 
            (liked.menu_id = mi.id AND liked.user_id = '{uid}')
        LEFT JOIN kueater.disliked_item disliked ON 
            (disliked.menu_id = mi.id AND disliked.user_id = '{uid}')
        LEFT JOIN kueater.saved_item saved ON 
            (saved.menu_id = mi.id AND saved.user_id = '{uid}')
        WHERE mi.name ILIKE '%{}%'
        GROUP BY 
            mi.id,
            mi.name, 
            price, 
            mi.image, 
            st.name, 
            lock
        "
    , data.query, uid=data.user);

    let menus = match sqlx::query(&menu_query).fetch_all(pg_pool).await {
        Err(e) => {
            println!("{}", e);
            return Err(Status::internal("Internal error"))
        }
        Ok(v) => v
    };

    for row in menus {
        search_results.insert(0, SearchEntry {
            id: row.get::<Uuid, &str>("id").to_string(),
            name: row.get("name"),
            price: row.get::<Decimal, &str>("price").to_f64().unwrap(),
            image: row.get("image"),
            stall_name: row.get("stall_name"),
            stall_lock: row.get("lock"),
            likes: row.get::<i64, &str>("likes").to_i32().unwrap(),
            liked_by_user: row.get::<bool, &str>("liked"),
            disliked_by_user: row.get::<bool, &str>("disliked"),
            saved_by_user: row.get::<bool, &str>("saved")
        });
    }

    search_results.dedup();

    let mut results: Vec<SearchResult> = vec![];
    for row in search_results {
        results.push(
            SearchResult {
                result: Some(
                    Item(CardedMenuItem {
                        item: Some(MenuItem {
                            uuid: row.id,
                            name: Some(
                                LocalizedString {
                                    content: row.name,
                                    locale: String::from("en")
                                }
                            ),
                            price: row.price,
                            ingredients: vec![],
                            image: row.image,
                            tags: vec![]
                        }),
                        stall_name: Some(
                            LocalizedString {
                                content: row.stall_name,
                                locale: String::from("en")
                            }
                        ),
                        stall_lock: row.stall_lock,
                        likes: row.likes,
                        liked_by_user: row.liked_by_user,
                        disliked_by_user: row.disliked_by_user,
                        favorite_by_user: row.saved_by_user
                    })
                )
            }
        );
    }

    let resp = SearchResponse {
        results: results,
        next_page_token: String::from("")
    };

    Ok(Response::new(resp))
}