use tonic::{Request, Response, Status};

use super::kueater::data::index::{
    GetMenuListingsRequest, GetMenuListingsResponse, CardedMenuItem,
    TopMenu, TopMenuRequest, SortStrategy
};

pub async fn get_menu_listing(
    request: Request<GetMenuListingsRequest>
) -> Result<Response<GetMenuListingsResponse>, Status> {

    println!("Running menu listings");

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