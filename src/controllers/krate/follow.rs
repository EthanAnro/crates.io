//! Endpoints for managing a per user list of followed crates

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::ok_true;
use crate::models::{Crate, Follow};
use crate::schema::*;
use crate::util::errors::{crate_not_found, AppResult};
use axum::extract::Path;
use axum::response::Response;
use axum::Json;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use serde_json::Value;

async fn follow_target(
    crate_name: &str,
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> AppResult<Follow> {
    let crate_id = Crate::by_name(crate_name)
        .select(crates::id)
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(crate_name))?;

    Ok(Follow { user_id, crate_id })
}

/// Handles the `PUT /crates/:crate_id/follow` route.
pub async fn follow(
    app: AppState,
    Path(crate_name): Path<String>,
    req: Parts,
) -> AppResult<Response> {
    let mut conn = app.db_write().await?;
    let user_id = AuthCheck::default().check(&req, &mut conn).await?.user_id();
    let follow = follow_target(&crate_name, &mut conn, user_id).await?;
    diesel::insert_into(follows::table)
        .values(&follow)
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;

    ok_true()
}

/// Handles the `DELETE /crates/:crate_id/follow` route.
pub async fn unfollow(
    app: AppState,
    Path(crate_name): Path<String>,
    req: Parts,
) -> AppResult<Response> {
    let mut conn = app.db_write().await?;
    let user_id = AuthCheck::default().check(&req, &mut conn).await?.user_id();
    let follow = follow_target(&crate_name, &mut conn, user_id).await?;
    diesel::delete(&follow).execute(&mut conn).await?;

    ok_true()
}

/// Handles the `GET /crates/:crate_id/following` route.
pub async fn following(
    app: AppState,
    Path(crate_name): Path<String>,
    req: Parts,
) -> AppResult<Json<Value>> {
    use diesel::dsl::exists;

    let mut conn = app.db_read_prefer_primary().await?;
    let user_id = AuthCheck::only_cookie()
        .check(&req, &mut conn)
        .await?
        .user_id();

    let follow = follow_target(&crate_name, &mut conn, user_id).await?;
    let following = diesel::select(exists(follows::table.find(follow.id())))
        .get_result::<bool>(&mut conn)
        .await?;

    Ok(Json(json!({ "following": following })))
}
