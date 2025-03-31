use std::collections::HashMap;
use std::future::{self, Future};
use std::{pin::Pin, str::FromStr};
use std::clone::Clone;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use sqlx::types::Uuid;
use sqlx::{query, PgPool, Row};
use tonic::{Request, Response, Status};
use tower::{BoxError, Layer, Service};
use http::{Request as HttpRequest, Response as HttpResponse, Uri};
use super::kueater_auth::auth_service_server::AuthService;
use super::kueater_auth::{AuthProcessRequest, AuthProcessResponse, LogoutProcessRequest, LogoutProcessResponse};

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
pub struct AuthLayer {
    google_auth_info: GoogleAuthClientInfo,
    pg_pool: PgPool
}

impl AuthLayer {
    pub fn new(google_auth_info: GoogleAuthClientInfo, pg_pool: PgPool) -> Self {
        Self {
            google_auth_info,
            pg_pool
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            google_auth_info: self.google_auth_info.clone(),
            http_client: Client::new(),
            pg_pool: self.pg_pool.clone()
        }
    }
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    google_auth_info: GoogleAuthClientInfo,
    http_client: Client,
    pg_pool: PgPool
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// Token based authentication middleware
impl<S, ReqBody, ResBody> Service<HttpRequest<ReqBody>> for AuthMiddleware<S> where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<BoxError> + Send + Sync,
    ReqBody: Send + 'static,
    ResBody: Send + 'static
{
    type Response = S::Response;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: HttpRequest<ReqBody>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let path = req.uri().path();

        // Skip auth service so this protects only the rest of API
        if path.contains("kueater.auth.AuthService") {
            return Box::pin(async move {
                inner.call(req).await.map_err(Into::into)
            });
        }

        let auth_header = req.headers().get(http::header::AUTHORIZATION);
        let token_result = match auth_header {
            Some(h) => {
                match h.to_str() {
                    Ok(s) if s.starts_with("Bearer ") => Ok(s[7..].to_string()),
                    Ok(_) => {
                        let status = Status::unauthenticated("Invalid authorization format");
                        Err(status.into())
                    },
                    Err(_) => {
                        let status = Status::unauthenticated("Invalid authorization header");
                        Err(status.into())
                    }
                }
            },
            None => {
                let status = Status::unauthenticated("Missing authorization header");
                Err(status.into())
            }
        };

        // Early return if token extraction failed
        let token = match token_result {
            Ok(t) => t,
            Err(e) => return Box::pin(future::ready(Err(e))),
        };
        
        let pg_pool = self.pg_pool.clone();

        Box::pin(async move {

            let query = sqlx::query_as::<_, (bool,)>(
                "
                SELECT
                COALESCE(BOOL_OR(token IS NOT NULL), FALSE) AS exist
                FROM kueater.google_access_token
                WHERE token = $1
                "
            )
            .bind(&token)
            .fetch_one(&pg_pool)
            .await;
        
            match query {
                Ok((exist,)) => {
                    if !exist {
                        let status = Status::unauthenticated("Invalid token");
                        return Err(status.into());
                    }
                    // Token is valid, proceed with the request
                    inner.call(req).await.map_err(Into::into)
                }
                Err(_) => {
                    let status = Status::internal("Database error");
                    Err(status.into())
                }
            }
        })
    }
}