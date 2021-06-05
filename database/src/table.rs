use async_trait::async_trait;

#[async_trait]
pub trait Table {
    async fn create_schema(&self) -> Result<(), sqlx::Error>;
}
