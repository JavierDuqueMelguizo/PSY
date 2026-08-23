use std::{fmt::Display, path::Path};


pub mod logging;


pub fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub async fn save_data(path: impl AsRef<Path>, data: &impl AsRef<[u8]>) -> Result<(), Box<dyn std::error::Error>>{
    tokio::fs::write(path, data).await?;
    Ok(())
}

pub async fn load_data(path: impl AsRef<Path> + Display + Clone) -> Option<Vec<u8>>{
    match tokio::fs::read(path.clone()).await{
        Ok(value) => Some(value),
        Err(e) => {
            eprintln!("Error leyendo fichero '{}'. Error: {}", path, e);
            None
        }
    }
}