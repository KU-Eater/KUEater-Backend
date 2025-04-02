use sqlx::PgPool;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};
use crate::AgentCommand;

use super::kueater::data::ku_eater_backend_server::KuEaterBackend;
use super::kueater::data::*;
use super::kueater::Empty;

pub type AgentCommandSender = mpsc::Sender<AgentCommand>;

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

pub type Recv<T> = Request<T>;
pub type Send<T> = Result<Response<T>, Status>;

#[macro_export]
macro_rules! no_impl {
    () => {
        return Err(tonic::Status::unimplemented("Unimplemented method"))
    }
}

#[tonic::async_trait]
impl KuEaterBackend for BackendService {

    async fn account_readiness(
        &self, request: Recv<AccountReadinessRequest>
    ) -> Send<AccountReadinessResponse> {
        super::after::account_readiness(&self.pg_pool, request).await
    }

    async fn create_account(
        &self, request: Recv<CreateAccountRequest>
    ) -> Send<Empty> {
        super::after::create_account(&self.pg_pool, &self.sender, request,).await
    }

    async fn get_preferences(
        &self, request: Recv<GetPreferencesRequest>
    ) -> Send<GetPreferencesResponse> {
        super::getters::get_preferences(&self.pg_pool, request).await
    }

    async fn get_menu_item(
        &self, request: Recv<GetMenuItemRequest>
    ) -> Send<types::MenuCardProps> {
        super::getters::get_menu_item(&self.pg_pool, request).await
    }

    async fn get_stall(
        &self, request: Recv<GetStallRequest>
    ) -> Send<types::StallDataTypeProps> {
        super::getters::get_stall(&self.pg_pool, request).await
    }

    async fn stall_items(
        &self, request: Recv<StallItemsRequest>
    ) -> Send<types::MenuCardGridConstructor> {
        super::getters::items_in_stall(&self.pg_pool, request).await
    }

    // Get 40 menu items sorted by like count
    async fn home_top_menu(
        &self, request: Recv<Empty>
    ) -> Send<home::TopMenuProps> {
        super::home::top_menu(&self.pg_pool, request).await
    }

    // Get 10 stalls from like count and review count averaged
    async fn home_top_stall(
        &self, request: Recv<Empty>
    ) -> Send<home::TopStallProps> {
        super::home::top_stall(&self.pg_pool, request).await
    }

    // Randomly choose a favorite dish of user,
    // then use it to find recommendations from reasoning with favorite dish.
    async fn home_infer_like(
        &self, request: Recv<home::InferLikeMsg>
    ) -> Send<home::InferLikeProps> {
        super::home::infer_like(&self.pg_pool, request).await
    }

    // Select randomly 8 menu from user recommendations, fresh account = empty list
    async fn home_for_you(
        &self, request: Recv<home::ForYouMsg>
    ) -> Send<home::ForYouProps> {
        super::home::for_you(&self.pg_pool, request).await
    }

    // Getting list of recommendations from highest score -> lowest (score must be higher than 5)
    // Fresh account -> gets menu by indexing in database
    async fn home_get_recommendations(
        &self, request: Recv<home::GetRecommendationsMsg>
    ) -> Send<home::RecommendationsList> {
        super::home::get_recommendations(&self.pg_pool, request).await
    }

    // The request fires once and retrieves all result,
    // Sends a message to channel to agent client and calculate vectors.
    // The vectors returned and we use PostgreSQL to get LIMIT 200 on menuitems which are closest to vectors.
    // The stalls are calculated by ranking the presence in search results for menuitems.
    async fn search(&self, request: Recv<search::SearchRequest>) -> Send<search::SearchResponse> {
        super::search::search(&self.pg_pool, &self.sender, request).await
    }

    async fn list_reviews(&self, request: Recv<review::ListReviewsRequest>) -> Send<review::ListReviewsResponse> {
        super::review::list_reviews(&self.pg_pool, request).await
    }

    async fn post_review(&self, request: Recv<review::PostReviewRequest>) -> Send<review::PostReviewResponse> {
        super::review::post_review(&self.pg_pool, request).await
    }

    async fn saved_items(&self, request: Recv<SavedItemsRequest>) -> Send<SavedItemsResponse> {
        super::saved::saved_items(&self.pg_pool, request).await
    }

    async fn saved_stalls(&self, request: Recv<SavedStallsRequest>) -> Send<SavedStallsResponse> {
        super::saved::saved_stalls(&self.pg_pool, request).await
    }

    // These are tally functions, which requires additional check for tally after it is complete.
    async fn like_item(&self, request: Recv<activity::LikeItemMsg>) -> Send<Empty> {
        super::activity::like_item(&self.pg_pool, request, &self.sender).await
    }

    async fn dislike_item(&self, request: Recv<activity::DislikeItemMsg>) -> Send<Empty> {
        super::activity::dislike_item(&self.pg_pool, request, &self.sender).await
    }

    async fn save_item(&self, request: Recv<activity::SaveItemMsg>) -> Send<Empty> {
        super::activity::save_item(&self.pg_pool, request, &self.sender).await
    }

    async fn like_stall(&self, request: Recv<activity::LikeStallMsg>) -> Send<Empty> {
        super::activity::like_stall(&self.pg_pool, request, &self.sender).await
    }

    async fn save_stall(&self, request: Recv<activity::SaveStallMsg>) -> Send<Empty> {
        super::activity::save_stall(&self.pg_pool, request, &self.sender).await
    }
    // End of tally functions

    // Refer to create account for no-headache
    async fn save_profile(&self, request: Recv<SaveProfileRequest>) -> Send<Empty> {
        super::profile::save_profile(&self.pg_pool, request).await
    }

    async fn save_preferences(&self, request: Recv<SavePreferencesRequest>) -> Send<Empty> {
        super::profile::save_preferences(&self.pg_pool, request).await
    }
}