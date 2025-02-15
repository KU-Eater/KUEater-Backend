use kueater::{LocalizedString, Ingredient, MenuItem};
use kueater::data::{GetMenuRequest, GetMenuResponse};
use kueater::data::ku_eater_backend_server::{KuEaterBackend, KuEaterBackendServer};
use tonic::transport::Server;
use tonic::{Status, Request, Response};

pub mod kueater {
    tonic::include_proto!("kueater");
    pub mod data {
        tonic::include_proto!("kueater.data");
        pub mod index {
            tonic::include_proto!("kueater.data.index");
        }
        pub mod search {
            tonic::include_proto!("kueater.data.search");
        }
    }
}

#[derive(Debug, Default)]
pub struct BackendService {}

#[tonic::async_trait]
impl KuEaterBackend for BackendService {

    async fn get_menu_item(
        &self,
        request: Request<GetMenuRequest>,
    ) -> Result<Response<GetMenuResponse>, Status> {
        let resp = GetMenuResponse {
            item: Some(MenuItem {
                uuid: request.into_inner().uuid,
                name: Some(
                    LocalizedString { content: "Chicken Wings".to_string() , locale: "en".to_string() }
                ),
                price: 30.0,
                ingredients: vec![
                    Ingredient {
                        uuid: "2".to_string(),
                        name: Some(
                            LocalizedString { content: "Chicken".to_string() , locale: "en".to_string() }
                        )
                    }
                ],
                image: "".to_string(),
                tags: vec![]
            })
        };
        Ok(Response::new(resp))
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