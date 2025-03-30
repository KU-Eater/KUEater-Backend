use http::Uri;
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
use agent::{command::{AgentCommand, Command}, kueater_agent::{self, ku_eater_embedding_agent_client::KuEaterEmbeddingAgentClient}};
use sqlx::{PgPool};
use tokio::{
    sync::mpsc
};
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use std::env::var;

mod service;
mod db;
mod middleware;
mod agent;

type AgentCommandSender = mpsc::Sender<AgentCommand>;

#[derive(Debug)]
pub struct BackendService {
    pg_pool: PgPool,
    sender: AgentCommandSender
}

impl BackendService {
    pub fn new(pg_pool: PgPool, sender: AgentCommandSender) -> Self {
        Self {
            pg_pool,
            sender
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
        service::search::search(&self.pg_pool, &self.sender, request).await
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

#[cfg(unix)]
async fn shutdown_signal_recv() -> std::io::Result<()> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();

    tokio::select! {
        _ = sigterm.recv() => Ok(()),
        _ = sigint.recv() => Ok(())
    }
}

#[cfg(windows)]
async fn shutdown_signal_recv() -> std::io::Result<()> {
    use tokio::signal::windows::{ctrl_c, ctrl_close};

    let mut ctrlc = ctrl_c().unwrap();
    let mut ctrlclose = ctrl_close().unwrap();

    tokio::select! {
        _ = ctrlc.recv() => Ok(()),
        _ = ctrlclose.recv() => Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    println!("Trying to connect to PostgreSQL database...");
    let pg: PgPool = db::connect(var("DATABASE_URL").expect("DATABASE_URL is not set or cannot be read")).await?;

    let sv_addr = "0.0.0.0:50051".parse()?;
    let agent_addr = var("AGENT_URL").unwrap_or(String::from("http://127.0.0.1:50052")).parse::<Uri>()?;

    println!("Starting gRPC server...");

    let (tx, mut rx) = mpsc::channel::<AgentCommand>(1024);

    let pg_inner = pg.clone();
    let server_tx = tx.clone();

    let sv = tokio::spawn(async move {
        let service = BackendService {
            pg_pool: pg_inner.clone(),
            sender: server_tx
        };

        let debug_svc = DebugService {
            pg_pool: pg_inner.clone()
        };

        Server::builder()
            .accept_http1(true)
            .layer(tower_http::cors::CorsLayer::very_permissive())
            .layer(GrpcWebLayer::new())
            .add_service(KuEaterBackendServer::new(service))
            .add_service(KuEaterDebugServer::new(debug_svc))
            .serve_with_shutdown(sv_addr, async {
                shutdown_signal_recv().await.ok();
            })
            .await.unwrap();
    });

    let _agt = tokio::spawn(async move {
        let mut client = KuEaterEmbeddingAgentClient::connect(agent_addr).await.expect(
            "Failed to connect to agent service"
        );

        println!("Connected to an agent service");

        while let Some(incoming) = rx.recv().await {
            match incoming.msg {
                Command::Search {query} => {
                    let request = Request::new(kueater_agent::GetEmbeddingRequest {
                        text: query.into()
                    });
                    let response = client.get_embedding(request).await.unwrap();
                    let vectors = response.into_inner().vectors;
                    incoming.tx.send(vectors).unwrap_or_else(|_| println!("Error while sending back search vectors"));
                },
                Command::Recommend {user_id} => {
                    let request = Request::new(kueater_agent::NewRecommendationsRequest {
                        user_id: user_id.into()
                    });
                    client.new_recommendations(request).await.unwrap();
                }
            }
        }
    });

    sv.await.unwrap();

    pg.close().await;

    Ok(())
}