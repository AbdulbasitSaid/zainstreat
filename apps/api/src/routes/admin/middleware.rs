use crate::error::AppError;
use crate::routes::admin::SESSION_USER_ID_KEY;
use axum::{extract::Request, middleware::Next, response::Response};
use tower_sessions::Session;

pub async fn require_admin(
    session: Session,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user_id: Option<i64> = session.get(SESSION_USER_ID_KEY).await?;
    if user_id.is_none() {
        return Err(AppError::Unauthorized);
    }
    Ok(next.run(request).await)
}
