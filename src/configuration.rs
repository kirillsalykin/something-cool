use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub cognito: CognitoConfig,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub addr: SocketAddr,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Deserialize)]
pub struct CognitoConfig {
    pub region: String,
    pub user_pool_id: String,
}

impl CognitoConfig {
    pub fn jwks_uri(&self) -> String {
        format!(
            "https://cognito-idp.{}.amazonaws.com/{}/.well-known/jwks.json",
            self.region, self.user_pool_id
        )
    }

    pub fn issuer(&self) -> String {
        format!(
            "https://cognito-idp.{}.amazonaws.com/{}",
            self.region, self.user_pool_id
        )
    }
}
