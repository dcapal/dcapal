//! `GET /`: service greeting.

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Service greeting", body = String)
    )
)]
/// Returns the service greeting used by the root health-style endpoint.
pub async fn root() -> &'static str {
    "Greetings from DCA-Pal APIs!"
}
