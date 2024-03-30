use std::{error::Error, sync::Arc, time::Duration};

use chrono::Utc;
use common::{Db, Task};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const MAX_CONCURRENT_TASKS: usize = 1;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let running_tasks = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));
    let db = Arc::new(Db::try_new(&database_url).await?);

    loop {
        let next_task = db.get_next_task_executable_at(Utc::now()).await?;
        match next_task {
            None => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Some(task) => {
                let permit = running_tasks.clone().acquire_owned().await?;
                let db = Arc::clone(&db);
                run_task(permit, db, task);
            }
        }
    }
}

fn run_task(permit: OwnedSemaphorePermit, db: Arc<Db>, task: Task) {
    tokio::spawn(async move {
        // Do Something
        task.run().await;
        let result = db.complete_task(task.id).await;
        if let Err(err) = result {
            // This should really never happen, but if it does, we want to know about it
            eprintln!("Failed to complete task: {:?}", err);
        }
        // We explicitly drop the permit here so that it's moved into the spawned task,
        // and will be released at the end.
        drop(permit);
    });
}
