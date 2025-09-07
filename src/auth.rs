use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

use crate::state::AppState;
use tracing::debug;

// Basic auth middleware; if credentials configured, enforce; otherwise allow.
pub async fn basic_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    if let Some(expected) = &state.basic_auth {
        debug!("basic auth required for this route");
        let auth = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        if let Some(auth) = auth
            && let Some(creds) = auth.strip_prefix("Basic ")
            && let Ok(bytes) = B64.decode(creds)
            && let Ok(text) = String::from_utf8(bytes)
            && text == expected.as_str()
        {
            // expected is "user:pass"
            // Optionally pass username to handlers
            let user = extract_user(expected);
            debug!(user = %user, "basic auth success");
            req.extensions_mut().insert(BasicUser(user));
            return Ok(next.run(req).await);
        }
        debug!("basic auth failed or missing header");
        return Err((StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    debug!("no basic auth configured; allowing request");
    Ok(next.run(req).await)
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct BasicUser(pub String);

fn extract_user(userpass: &str) -> String {
    match userpass.split_once(':') {
        Some((u, _)) => u.to_string(),
        None => String::new(),
    }
}
