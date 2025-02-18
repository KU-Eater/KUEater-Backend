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
        pub mod review {
            tonic::include_proto!("kueater.data.review");
        }
    }
    pub mod debug {
        tonic::include_proto!("kueater.debug");
        pub mod datagen {
            tonic::include_proto!("kueater.debug.datagen");
        }
    }
}

pub mod fetch;
pub mod index;
pub mod search;
pub mod review;