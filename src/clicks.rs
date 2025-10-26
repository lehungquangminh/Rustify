use std::time::Duration;
use sqlx::PgPool;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::error;

pub fn start_click_flusher(pool: PgPool, mut rx: mpsc::UnboundedReceiver<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
        use std::collections::HashMap;
        let mut counts: HashMap<String, i64> = HashMap::new();
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if counts.is_empty() { continue; }
                    let data: Vec<(String, i64)> = counts.drain().collect();
                    let mut tx = match pool.begin().await { Ok(t) => t, Err(e) => { error!(?e, "tx"); continue; } };
                    for (alias, n) in data {
                        let _ = sqlx::query("INSERT INTO clicks(alias, ts, n) VALUES ($1, now(), $2)")
                            .bind(&alias)
                            .bind(n)
                            .execute(&mut tx)
                            .await;
                    }
                    let _ = tx.commit().await;
                }
                msg = rx.recv() => {
                    if let Some(a) = msg { *counts.entry(a).or_default() += 1; }
                    else { break; }
                }
            }
        }
    })
}
