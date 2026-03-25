use crate::error::ApiError;
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

const JWKS_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Serialize)]
pub struct AuthContext {
    pub user_id: String,
    pub email: Option<String>,
    pub role: Option<String>,
}

#[derive(Clone)]
pub struct AuthVerifier {
    issuer: String,
    jwks_url: String,
    client: reqwest::Client,
    cache: Arc<RwLock<JwksCache>>,
}

#[derive(Debug, Default)]
struct JwksCache {
    fetched_at: Option<Instant>,
    keys: HashMap<String, DecodingKey>,
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    sub: String,
    email: Option<String>,
    role: Option<String>,
}

impl AuthVerifier {
    pub fn new(issuer: impl Into<String>, jwks_url: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            jwks_url: jwks_url.into(),
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(JwksCache::default())),
        }
    }

    pub async fn verify_headers(&self, headers: &HeaderMap) -> Result<AuthContext, ApiError> {
        let token = parse_bearer_token(headers)?;
        self.verify_token(token).await
    }

    async fn verify_token(&self, token: &str) -> Result<AuthContext, ApiError> {
        let header = decode_header(token).map_err(|_| unauthorized())?;
        let kid = header.kid.as_deref().ok_or_else(unauthorized)?;
        let decoding_key = self.decoding_key_for(kid).await?;

        let mut validation = Validation::new(header.alg);
        validation.set_required_spec_claims(&["aud", "exp", "iss", "sub"]);
        validation.set_audience(&["authenticated"]);
        validation.set_issuer(&[self.issuer.as_str()]);

        let claims = decode::<JwtClaims>(token, &decoding_key, &validation)
            .map_err(|_| unauthorized())?
            .claims;

        claims_to_context(claims)
    }

    async fn decoding_key_for(&self, kid: &str) -> Result<DecodingKey, ApiError> {
        if let Some(key) = self.cached_key(kid)? {
            return Ok(key);
        }

        self.refresh_jwks().await?;

        self.cached_key(kid)?.ok_or_else(unauthorized)
    }

    fn cached_key(&self, kid: &str) -> Result<Option<DecodingKey>, ApiError> {
        let cache = self.cache.read().map_err(|_| auth_unavailable())?;

        if !cache.is_fresh() {
            return Ok(None);
        }

        Ok(cache.keys.get(kid).cloned())
    }

    async fn refresh_jwks(&self) -> Result<(), ApiError> {
        let jwk_set = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|_| auth_unavailable())?
            .error_for_status()
            .map_err(|_| auth_unavailable())?
            .json::<JwkSet>()
            .await
            .map_err(|_| auth_unavailable())?;

        let mut keys = HashMap::new();
        for jwk in jwk_set.keys {
            let Some(kid) = jwk.common.key_id.clone() else {
                continue;
            };
            let Ok(key) = DecodingKey::from_jwk(&jwk) else {
                continue;
            };
            keys.insert(kid, key);
        }

        if keys.is_empty() {
            return Err(auth_unavailable());
        }

        let mut cache = self.cache.write().map_err(|_| auth_unavailable())?;
        cache.fetched_at = Some(Instant::now());
        cache.keys = keys;
        Ok(())
    }
}

impl JwksCache {
    fn is_fresh(&self) -> bool {
        self.fetched_at
            .is_some_and(|fetched_at| fetched_at.elapsed() < JWKS_CACHE_TTL)
    }
}

pub async fn require_bearer_auth(
    State(auth): State<AuthVerifier>,
    mut request: Request,
    next: Next,
) -> Response {
    match auth.verify_headers(request.headers()).await {
        Ok(context) => {
            request.extensions_mut().insert(context);
            next.run(request).await
        }
        Err(error) => error.into_response(),
    }
}

fn parse_bearer_token<'a>(headers: &'a HeaderMap) -> Result<&'a str, ApiError> {
    let header_value = headers
        .get(AUTHORIZATION)
        .ok_or_else(unauthorized)?
        .to_str()
        .map_err(|_| unauthorized())?;

    let (scheme, token) = header_value.split_once(' ').ok_or_else(unauthorized)?;
    let token = token.trim();

    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() {
        return Err(unauthorized());
    }

    Ok(token)
}

fn claims_to_context(claims: JwtClaims) -> Result<AuthContext, ApiError> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| unauthorized())?;

    Ok(AuthContext {
        user_id: user_id.to_string(),
        email: claims.email,
        role: claims.role,
    })
}

fn unauthorized() -> ApiError {
    ApiError::unauthorized("Authentication required.")
}

fn auth_unavailable() -> ApiError {
    ApiError::internal("Authentication is temporarily unavailable.")
}

#[cfg(test)]
mod tests {
    use super::{claims_to_context, parse_bearer_token, JwtClaims};
    use axum::{
        http::{header::AUTHORIZATION, HeaderMap, HeaderValue, StatusCode},
        response::IntoResponse,
    };

    #[test]
    fn parse_bearer_token_rejects_missing_header() {
        let headers = HeaderMap::new();

        let response = parse_bearer_token(&headers).unwrap_err().into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn parse_bearer_token_rejects_non_bearer_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc123"));

        let response = parse_bearer_token(&headers).unwrap_err().into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn claims_to_context_rejects_invalid_sub() {
        let claims = JwtClaims {
            sub: "not-a-uuid".to_owned(),
            email: Some("user@example.com".to_owned()),
            role: Some("authenticated".to_owned()),
        };

        let response = claims_to_context(claims).unwrap_err().into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
