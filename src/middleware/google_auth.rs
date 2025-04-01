use std::collections::HashMap;
use std::future::{self, Future};
use std::{pin::Pin, str::FromStr};
use std::clone::Clone;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;
use sqlx::{query, PgPool, Row};
use tonic::body::BoxBody;
use tonic::transport::Body;
use tonic::{async_trait, Request, Response, Status};
use tonic_middleware::RequestInterceptor;
use super::kueater_auth::auth_service_server::AuthService;
use super::kueater_auth::{AuthProcessRequest, AuthProcessResponse, LogoutProcessRequest, LogoutProcessResponse};

use tonic::codegen::http::{Request as Rq, Response as Rp};

// Follows Google OIDC auth process

const CERT_LINK: &str = "https://www.googleapis.com/oauth2/v3/certs";
const ISSUER_LINK: &str = "accounts.google.com";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

// reference: https://www.googleapis.com/oauth2/v3/certs
#[derive(Debug, Deserialize, Serialize)]
struct JwkKey {
    e: String,
    kty: String,
    kid: String,
    n: String,
    alg: String,
    #[serde(rename = "use")]
    usage: String
}

#[derive(Debug, Deserialize, Serialize)]
struct JwkCerts {
    keys: Vec<JwkKey>
}

// reference: https://developers.google.com/identity/openid-connect/openid-connect#an-id-tokens-payload
#[derive(Debug, Deserialize, Serialize)]
struct GoogleClaims {
    aud: String,    // Audience ID
    exp: i64,       // Expiry
    iss: String,    // Issuer URI
    sub: String,    // User identifier
    email: String,
    email_verified: bool,
    azp: Option<String>,    // Client ID of presenter, maybe required?
}

#[derive(Debug, Clone)]
pub struct GoogleAuthClientInfo {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String
}

#[derive(Debug, Deserialize)]
struct GoogleExchange {
    access_token: String,
    id_token: String
}

pub struct AuthServiceImpl {
    http_client: Client,
    pg_pool: PgPool,
    google_auth_info: GoogleAuthClientInfo
}

impl AuthServiceImpl {
    pub fn new(pg_pool: PgPool, google_auth_info: GoogleAuthClientInfo) -> Self {
        Self {
            http_client: Client::new(),
            pg_pool,
            google_auth_info
        }
    }

    async fn get_google_jwk(&self) -> Result<JwkCerts, Status> {
        let certs = self.http_client
            .get(CERT_LINK)
            .send()
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch Google JWK certs: {}", e)))?
            .json::<JwkCerts>()
            .await
            .map_err(|e| Status::internal(format!("Failed to parse Google JWK certs: {}", e)))?;

        Ok(certs)
    }

    async fn validate_google_id_token(&self, token: &str) -> Result<GoogleClaims, Status> {
        let certs = self.get_google_jwk().await?;

        // Extract key ID from token header
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| Status::invalid_argument(format!("Invalid token header: {}", e)))?;

        let kid = header.kid.ok_or_else(|| Status::invalid_argument(format!("Token missing key ID")))?;

        // Find matching key
        let key = certs.keys.iter()
            .find(|k| k.kid == kid)
            .ok_or_else(|| Status::internal(format!("No matching key found")))?;

        // Create decode key
        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
            .map_err(|e| Status::internal(format!("Failed to create decoding key: {}", e)))?;

        // Validation
        let mut validator = Validation::new(
            Algorithm::from_str(&key.alg)
            .map_err(|e| Status::invalid_argument(format!("Cannot get algorithm from alg: {}", e)))?
        );
        validator.set_audience(&[&self.google_auth_info.client_id]);
        validator.set_issuer(&[format!("https://{}", ISSUER_LINK), ISSUER_LINK.to_string()]);

        let claims = decode::<GoogleClaims>(token, &decoding_key, &validator)
            .map_err(|e| Status::internal(format!("Token validation failed: {}", e)))?
            .claims;

        Ok(claims)
    } 
}

// TODO: Implement Tonic Service for authorization
// TODO: Add function within the service to get user email
// TODO: Add function to validate token and retrieve access token
#[tonic::async_trait]
impl AuthService for AuthServiceImpl {

    async fn auth_process(
        &self, request: Request<AuthProcessRequest>
    ) -> Result<Response<AuthProcessResponse>, Status> {
        let data = request.into_inner();
        let code = data.code;
        let error = Status::aborted("Google auth failed");

        if code.is_empty() {
            return Err(Status::aborted("No Google code"))
        }

        // Start Google auth process
        let mut params = HashMap::new();
        params.insert("code", code);
        params.insert("client_id", self.google_auth_info.client_id.clone());
        params.insert("client_secret", self.google_auth_info.client_secret.clone());
        params.insert("redirect_uri", self.google_auth_info.redirect_uri.clone());
        params.insert("grant_type", String::from("authorization_code"));

        let resp = self.http_client.post(TOKEN_ENDPOINT)
            .form(&params)
            .send()
            .await.map_err(|_| Status::internal("Cannot send to token url"))?;

        let resp = resp.json::<GoogleExchange>()
            .await.map_err(|e| Status::internal(format!("Cannot retrieve tokens: {:?}", e)))?;

        // We can now pass the id token to validate
        let claims = self.validate_google_id_token(&resp.id_token).await?;

        // Database calls;
            // Do we have user profile with existing email?
        let _query = format!(
            "SELECT id, email FROM kueater.userprofile WHERE email = '{}'",
             claims.email
        );

        match sqlx::query(&_query).fetch_one(&self.pg_pool).await {
            Err(_) => {
                // Not existent, create a new user profile
                let mut tx = self.pg_pool.begin().await.map_err(|_| error.clone())?;
                
                let __query = format!(
                    "
                    INSERT INTO kueater.userprofile (name, email)
                    VALUES ('', '{}') RETURNING id
                    ", claims.email
                );
                let result = sqlx::query(&__query)
                .fetch_one(&mut *tx).await.map_err(|_| Status::internal("Database operations error"))?;

                // UUID of user profile
                let uid = result.get::<Uuid, &str>("id");
                let __query = format!(
                    "
                    INSERT INTO kueater.google_access_token (token, user_id) VALUES
                    ('{}', '{}')
                    ", resp.access_token, uid.to_string()
                );
                sqlx::query(&__query)
                .execute(&mut *tx).await.map_err(|_| Status::internal("Database operations error"))?;

                tx.commit().await.map_err(|_| error.clone())?;

                return Ok(Response::new(AuthProcessResponse {
                    token: resp.access_token,
                    user_id: uid.to_string()
                }));
            }
            Ok(r) => {
                // Exist
                let uid = r.get::<Uuid, &str>("id");

                let mut tx = self.pg_pool.begin().await.map_err(|_| error.clone())?;
                // Ensure that old access token is deleted,
                let __query = format!(
                    "
                    DELETE FROM kueater.google_access_token
                    WHERE user_id = '{}'
                    ", uid.to_string()
                );
                sqlx::query(&__query).execute(&mut *tx).await.map_err(
                    |e| Status::internal(format!("Database operations error: {}", e))
                )?;

                let __query = format!(
                    "
                    INSERT INTO kueater.google_access_token (token, user_id) VALUES
                    ('{}', '{}')
                    ", resp.access_token, uid.to_string() 
                );
                sqlx::query(&__query)
                .execute(&mut *tx).await.map_err(
                    |e| Status::internal(format!("Database operations error: {}", e))
                )?;

                tx.commit().await.map_err(|_| error.clone())?;

                return Ok(Response::new(AuthProcessResponse {
                    token: resp.access_token,
                    user_id: uid.to_string(),
                }));
            }
        };
    }

    async fn logout_process(
        &self, request: Request<LogoutProcessRequest>
    ) -> Result<Response<LogoutProcessResponse>, Status> {
        Err(Status::unimplemented("Unimplemented"))
    }

}

#[derive(Clone)]
pub struct AuthInterceptor {
    google_auth_info: GoogleAuthClientInfo,
    pg_pool: PgPool
}

impl AuthInterceptor {
    pub fn new(google_auth_info: GoogleAuthClientInfo, pg_pool: PgPool) -> Self {
        Self {
            google_auth_info,
            pg_pool
        }
    }
}

#[async_trait]
impl RequestInterceptor for AuthInterceptor {
    async fn intercept(&self, mut req: Rq<BoxBody>) -> Result<Rq<BoxBody>, Status> {
        let token_result = match req.headers().get("authorization") {
            Some(h) => {
                match h.to_str() {
                    Ok(token) if token.starts_with("Bearer ") => Ok(token[7..].to_string()),
                    Ok(t) => {
                        println!("{}", t);
                        Err(Status::unauthenticated("Invalid token format"))
                    }
                    Err(_) => Err(Status::unauthenticated("Invalid header"))
                }
            } 
            _ => Err(Status::unauthenticated("Unauthenticated"))
        };

        let token = match token_result {
            Ok(t) => t,
            Err(e) => return Err(e)
        };

        let pg_pool = self.pg_pool.clone();

        let query = sqlx::query_as::<_, (Uuid,)>(
            "
            SELECT
            user_id
            FROM kueater.google_access_token
            WHERE token = $1
            "
        )
        .bind(&token)
        .fetch_optional(&pg_pool)
        .await;

        match query {
            Ok(row) => {
                match row {
                    Some((id,)) => {
                        let extensions = req.extensions_mut();
                        extensions.insert(crate::service::UserContext {
                            user_id: id.to_string()
                        });
                        Ok(req)
                    }
                    None => {
                        return Err(Status::unauthenticated("Invalid token"))
                    }
                }
            }
            Err(e) => {
                println!("{}", e);
                Err(Status::internal("Database error"))
            }
        }
    }
}