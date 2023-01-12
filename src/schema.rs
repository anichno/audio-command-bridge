use sqlx::types::time::OffsetDateTime;

pub struct User {
    pub id: i32,
    pub email: String,
}

pub struct AccessToken {
    pub user_id: i32,
    pub token: String,
    pub token_expiration: OffsetDateTime,
    pub refresh_token: String,
    pub refresh_token_expiration: OffsetDateTime,
}
