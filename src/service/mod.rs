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

pub mod fetch;
pub mod index;
pub mod search;