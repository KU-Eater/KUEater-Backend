use service::kueater::data::{
    index::{GetMenuListingsRequest, GetMenuListingsResponse, TopMenu, TopMenuRequest}, ku_eater_backend_server::{KuEaterBackend, KuEaterBackendServer}, search::{SearchRequest, SearchResponse}, GetMenuRequest, GetMenuResponse, GetReviewRequest, GetReviewResponse, GetStallRequest, GetStallResponse
};
use tonic::{transport::Server, Request, Response, Status};

mod service;

#[derive(Default, Debug)]
pub struct BackendService {}

#[tonic::async_trait]
impl KuEaterBackend for BackendService {

    async fn index_get_menu_listings(
        &self, request: Request<GetMenuListingsRequest>
    ) -> Result<Response<GetMenuListingsResponse>, Status> {
        service::index::get_menu_listing(request).await
    }

    async fn index_top_menu(
        &self, request: Request<TopMenuRequest>
    ) -> Result<Response<TopMenu>, Status> {
        service::index::index_top_menu(request).await
    }

    async fn search(
        &self, request: Request<SearchRequest>
    ) -> Result<Response<SearchResponse>, Status> {
        service::search::search(request).await
    }

    //TODO: Add basic utilities for database fetching

    async fn get_menu_item(
        &self, request: Request<GetMenuRequest>
    ) -> Result<Response<GetMenuResponse>, Status> {
        Ok(Response::new(GetMenuResponse {
            item: None
        }))
    }

    async fn get_stall(
        &self, request: Request<GetStallRequest>
    ) -> Result<Response<GetStallResponse>, Status> {
        Ok(Response::new(GetStallResponse {
            stall: None
        }))
    }

    async fn get_review(
        &self, request: Request<GetReviewRequest>
    ) -> Result<Response<GetReviewResponse>, Status> {
        Ok(Response::new(GetReviewResponse { 
            review: None
        }))
    }

}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let service = BackendService::default();

    Server::builder()
        .add_service(KuEaterBackendServer::new(service))
        .serve(addr)
        .await?;
    
    Ok(())
}