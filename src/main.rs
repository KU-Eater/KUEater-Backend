use service::kueater::data::{
    index::{GetMenuListingsRequest, GetMenuListingsResponse, TopMenu, TopMenuRequest, TopStall, TopStallRequest},
    ku_eater_backend_server::{KuEaterBackend, KuEaterBackendServer}, search::{SearchRequest, SearchResponse},
    GetMenuRequest, GetMenuResponse, GetReviewRequest, GetReviewResponse, GetStallRequest, GetStallResponse,
    review::{PostReviewRequest, PostReviewResponse, ListReviewsRequest, ListReviewsResponse}
};
use service::kueater::debug::{
    datagen::{CreateTestUserProfileRequest, CreateTestUserProfileResponse},
    ku_eater_debug_server::{KuEaterDebug, KuEaterDebugServer}
};
use sqlx::{PgPool};
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tokio::signal::ctrl_c;
use std::env::var;

mod service;
mod db;

#[derive(Debug)]
pub struct BackendService {
    pg_pool: PgPool
}

impl BackendService {
    pub fn new(pg_pool: PgPool) -> Self {
        Self {
            pg_pool
        }
    }
}

#[tonic::async_trait]
impl KuEaterBackend for BackendService {

    async fn index_get_menu_listings(
        &self, request: Request<GetMenuListingsRequest>
    ) -> Result<Response<GetMenuListingsResponse>, Status> {
        service::index::get_menu_listing(&self.pg_pool, request).await
    }

    async fn index_top_menu(
        &self, request: Request<TopMenuRequest>
    ) -> Result<Response<TopMenu>, Status> {
        service::index::index_top_menu(&self.pg_pool, request).await
    }

    async fn index_top_stall(
        &self, request: Request<TopStallRequest>
    ) -> Result<Response<TopStall>, Status> {
        service::index::index_top_stall(&self.pg_pool, request).await
    }

    async fn search(
        &self, request: Request<SearchRequest>
    ) -> Result<Response<SearchResponse>, Status> {
        service::search::search(&self.pg_pool, request).await
    }

    async fn list_reviews(
        &self, request: Request<ListReviewsRequest>
    ) -> Result<Response<ListReviewsResponse>, Status> {
        service::review::list_reviews(&self.pg_pool, request).await
    }

    async fn post_review(
        &self, request: Request<PostReviewRequest>
    ) -> Result<Response<PostReviewResponse>, Status> {
        service::review::post_review(&self.pg_pool, request).await
    }

    async fn get_menu_item(
        &self, request: Request<GetMenuRequest>
    ) -> Result<Response<GetMenuResponse>, Status> {
        service::fetch::get_menu_item(&self.pg_pool, request).await
    }

    async fn get_stall(
        &self, request: Request<GetStallRequest>
    ) -> Result<Response<GetStallResponse>, Status> {
        service::fetch::get_stall(&self.pg_pool, request).await
    }

    async fn get_review(
        &self, _request: Request<GetReviewRequest>
    ) -> Result<Response<GetReviewResponse>, Status> {
        Err(Status::unimplemented("Unimplemented"))
    }

}

// ---
#[derive(Debug)]
pub struct DebugService {
    pg_pool: PgPool
}

impl DebugService {
    pub fn new(pg_pool: PgPool) -> Self {
        Self {
            pg_pool
        }
    }
}

#[tonic::async_trait]
impl KuEaterDebug for DebugService {
    async fn create_test_user_profile(
        &self, request: Request<CreateTestUserProfileRequest>
    ) -> Result<Response<CreateTestUserProfileResponse>, Status> {
        Err(Status::unimplemented("Unimplemented method"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    println!("Trying to connect to PostgreSQL database...");
    let pg: PgPool = db::connect(var("DATABASE_URL").expect("DATABASE_URL is not set or cannot be read")).await?;

    let addr = "0.0.0.0:50051".parse()?;
    let service = BackendService {
        pg_pool: pg.clone()
    };
    
    let debug_svc = DebugService {
        pg_pool: pg.clone()
    };

    println!("Starting gRPC server...");

    Server::builder()
        .accept_http1(true)
        .layer(tower_http::cors::CorsLayer::very_permissive())
        .layer(GrpcWebLayer::new())
        .add_service(KuEaterBackendServer::new(service))
        .add_service(KuEaterDebugServer::new(debug_svc))
        .serve_with_shutdown(addr, async {
            ctrl_c().await.ok();
        })
        .await?;

    pg.close().await;

    Ok(())
}