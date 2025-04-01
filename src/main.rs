use http::Uri;
use agent::{command::{AgentCommand, Command}, kueater_agent::{self, ku_eater_embedding_agent_client::KuEaterEmbeddingAgentClient}};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tonic_middleware::InterceptorFor;
use std::env::var;
use dotenv::dotenv;

use middleware::{google_auth::{GoogleAuthClientInfo}, kueater_auth::{self, auth_service_server::{AuthService, AuthServiceServer}}};
use service::kueater::debug::{
    datagen::{CreateTestUserProfileRequest, CreateTestUserProfileResponse},
    ku_eater_debug_server::{KuEaterDebug, KuEaterDebugServer}
};
use service::kueater::data::ku_eater_backend_server::KuEaterBackendServer;
use service::backend::BackendService;

mod service;
mod db;
mod middleware;
mod agent;

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

    dotenv().ok();

    println!("Trying to connect to PostgreSQL database...");
    let pg: PgPool = db::connect(var("DATABASE_URL").expect("DATABASE_URL is not set or cannot be read")).await?;

    let sv_addr = "0.0.0.0:50051".parse()?;
    let agent_addr = var("AGENT_URL").unwrap_or(String::from("http://127.0.0.1:50052")).parse::<Uri>()?;

    let google_auth_info = GoogleAuthClientInfo {
        client_id: var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID not set"),
        client_secret: var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET not set"),
        redirect_uri: var("GOOGLE_REDIRECT_URI").expect("GOOGLE_REDIRECT_URI not set")
    };

    println!("Starting gRPC server...");

    let (tx, mut rx) = mpsc::channel::<AgentCommand>(1024);

    let pg_inner = pg.clone();
    let server_tx = tx.clone();

    let sv = tokio::spawn(async move {
        let service = BackendService::new(pg_inner.clone(), server_tx);

        let debug_svc = DebugService {
            pg_pool: pg_inner.clone()
        };

        let auth_svc = middleware::google_auth::AuthServiceImpl::new(pg_inner.clone(), google_auth_info.clone());
        
        let interceptor = middleware::google_auth::AuthInterceptor::new(
            google_auth_info, pg_inner.clone());

        Server::builder()
            .accept_http1(true)
            .layer(tower_http::cors::CorsLayer::very_permissive())
            .layer(GrpcWebLayer::new())
            .add_service(AuthServiceServer::new(auth_svc))
            .add_service(InterceptorFor::new(KuEaterBackendServer::new(service), interceptor))
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
                    incoming.tx.unwrap().send(vectors).unwrap_or_else(|_| println!("Error while sending back search vectors"));
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