use crate::app::infra::claim::Claims;
use crate::error::{DcaError, Result};
use crate::ports::inbound::service;
use crate::AppContext;
use axum::extract::State;

/// Get user profile information.
#[utoipa::path(
    get,
    path = "/api/v1/user/profile",
    responses(
        (status = 200, description = "Success get user profile", body = [ProfileResponse]),
        (status = 401, description = "Unauthorized user", body = [AppResponseError]),
        (status = 500, description = "Internal server error", body = [AppResponseError])
    ),
    security(("jwt" = []))
)]
pub async fn get_profile(
    State(ctx): State<AppContext>,
    claims: Claims,
) -> Result<Json<ProfileResponse>> {
    //info!("Get profile user id: {}.", user.uid);
    match service::user::get_profile(&state, user.uid).await {
        Ok(resp) => {
            info!("Success get profile user: {}.", user.uid);
            Ok(Json(resp))
        }
        Err(e) => {
            warn!("Unsuccessfully get profile user: {e:?}.");
            Err(e)
        }
    }
}
