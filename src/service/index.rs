use sqlx::PgPool;
use tonic::{Request, Response, Status};

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
    
    for row in result {
        println!("{:?}", row);
    }

    let data = GetMenuListingsResponse {
        items: vec![],
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