pub mod kueater {
    tonic::include_proto!("kueater");
    pub mod data {
        tonic::include_proto!("kueater.data");
        pub mod types {
            tonic::include_proto!("kueater.data.types");
        }
        pub mod home {
            tonic::include_proto!("kueater.data.home");
        }
        pub mod search {
            tonic::include_proto!("kueater.data.search");
        }
        pub mod review {
            tonic::include_proto!("kueater.data.review");
        }
        pub mod activity {
            tonic::include_proto!("kueater.data.activity");
        }
    }
    pub mod debug {
        tonic::include_proto!("kueater.debug");
        pub mod datagen {
            tonic::include_proto!("kueater.debug.datagen");
        }
    }
}

#[derive(Clone)]
pub struct UserContext {
    pub user_id: String,
}

mod after;
mod getters;
mod home;
mod search;
mod review;
mod saved;
mod activity;
mod profile;
pub mod backend;