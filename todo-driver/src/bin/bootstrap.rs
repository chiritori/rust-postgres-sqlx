use todo_driver::models;
use todo_driver::startup::{init_app, startup};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_app();

    let db = models::Db::default();
    let _ = startup(db).await;

    Ok(())
}
