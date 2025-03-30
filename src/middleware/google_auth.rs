use std::future::{self, Future};
use std::{pin::Pin, str::FromStr};
use std::clone::Clone;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use tonic::Status;
use tower::{BoxError, Layer, Service};
use http::{Request as HttpRequest, Response as HttpResponse};

// Follows Google OIDC auth process

const CERT_LINK: &str = "https://www.googleapis.com/oauth2/v3/certs";
const ISSUER_LINK: &str = "accounts.google.com";

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

struct AuthServiceImpl {
    http_client: Client,
    google_client_id: String
}

impl AuthServiceImpl {
    fn new(google_client_id: String) -> Self {
        Self {
            http_client: Client::new(),
            google_client_id
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
        validator.set_audience(&[&self.google_client_id]);
        validator.set_issuer(&[format!("https://{}", ISSUER_LINK), ISSUER_LINK.to_string()]);

        let claims = decode::<GoogleClaims>(token, &decoding_key, &validator)
            .map_err(|e| Status::internal(format!("Token validation failed: {}", e)))?
            .claims;

        Ok(claims)
    } 
}

// TODO: Implement Tonic Service for autheorization
// TODO: Add function within the service to get user email
// TODO: Add function to validate token and retrieve access token

#[derive(Clone)]
struct AuthLayer {
    google_client_id: String
}

impl AuthLayer {
    fn new(google_client_id: String) -> Self {
        Self {
            google_client_id
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            google_client_id: self.google_client_id.clone(),
            http_client: Client::new()
        }
    }
}

#[derive(Clone)]
struct AuthMiddleware<S> {
    inner: S,
    google_client_id: String,
    http_client: Client
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

        // Make sure the client has auth header
        let auth_header = match req.headers().get("authorization") {
            Some(h) => h,
            None => {
                let status = Status::unauthenticated("Missing authorization header");
                return Box::pin(future::ready(Err(status.into())));
            }
        };

        // Verify token
        let auth_val = match auth_header.to_str() {
            Ok(s) => s,
            Err(_) => {
                let status = Status::unauthenticated("Invalid authorization header");
                return Box::pin(future::ready(Err(status.into())));
            }
        };

        if !auth_val.starts_with("Bearer ") {
            let status = Status::unauthenticated("Invalid authorization format");
            return Box::pin(future::ready(Err(status.into())));
        }

        // Extract token
        let token = &auth_val[7..];

        // TODO: Validate token within the database

        Box::pin(async move {
            inner.call(req).await.map_err(Into::into)
        })

    }
}