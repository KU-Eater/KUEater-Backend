use tonic::{Request, Response, Status};

use super::kueater::data::search::{
    SearchRequest, SearchResponse, CardedMenuItem, SortStrategy
};

pub async fn search(
    request: Request<SearchRequest>
) -> Result<Response<SearchResponse>, Status> {

    println!("Running search");

    let resp = SearchResponse {
        results: vec![],
        next_page_token: String::from("")
    };

    Ok(Response::new(resp))
}