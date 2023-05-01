use sqlx::{postgres::PgPoolOptions, Pool, Postgres};


pub struct AppState {
    pub db: Pool<Postgres>,
}

