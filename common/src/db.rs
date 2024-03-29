use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::query;
use sqlx::{postgres::PgPoolOptions, query_as};
use thiserror::Error;
use uuid::Uuid;

use crate::filter::Filter;
use crate::{Task, TaskKind, TaskStatus};

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Unable to connect to the database: {0}")]
    ConnectionError(#[from] sqlx::Error),
    #[error("Unable to generate task payload: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Invalid Taske Id specified: {0}")]
    InvalidTaskId(#[from] uuid::Error),
    #[error("Task {0} is not pending {1}")]
    TaskNotPending(Uuid, TaskStatus),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),
    #[error("Invalid status: {0}")]
    InvalidStatus(String),
    #[error("Invalid kind: {0}")]
    InvalidKind(String),
}

#[derive(Debug, Clone)]
pub struct Db {
    pool: sqlx::PgPool,
}

impl Db {
    pub async fn try_new(connection: &str) -> Result<Self, DbError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(connection)
            .await?;

        Ok(Self { pool })
    }

    #[cfg(test)]
    pub(crate) async fn clear_database(&self) -> Result<(), DbError> {
        query!("TRUNCATE TABLE tasks").execute(&self.pool).await?;

        Ok(())
    }

    pub async fn add_task(&self, task: &Task) -> Result<(), DbError> {
        query!(
            "INSERT INTO tasks (id, kind, process_at, status) VALUES ($1, $2, $3, $4)",
            task.id,
            task.kind as TaskKind,
            task.process_at,
            task.status as TaskStatus
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn complete_task(&self, id: Uuid) -> Result<(), DbError> {
        let result = query!(
            "UPDATE tasks SET status = $1 WHERE id = $2 AND status = $3",
            TaskStatus::Completed as TaskStatus,
            id,
            TaskStatus::InProgress as TaskStatus
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::TaskNotPending(id, TaskStatus::InProgress));
        }

        Ok(())
    }

    pub async fn get_next_task_executable_at(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<Task>, DbError> {
        let results = query_as!(
            Task,
            r#"
            WITH cte AS (
                SELECT id FROM tasks WHERE
             status = $1 AND process_at <= $2
             ORDER BY process_at LIMIT 1
            )
            UPDATE tasks
              SET status = $3
              FROM cte
              WHERE tasks.id = cte.id
            RETURNING tasks.id, status as "status: TaskStatus", kind as "kind: TaskKind", process_at"#,
            TaskStatus::Pending as TaskStatus,
            now,
            TaskStatus::InProgress as TaskStatus,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(results)
    }

    pub async fn get_tasks(&self) -> Result<Vec<Task>, DbError> {
        let results = query_as!(
            Task,
            r#"
            SELECT id, status as "status: TaskStatus", kind as "kind: TaskKind", process_at
            FROM tasks
            ORDER BY process_at
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }

    pub async fn get_filtered_tasks(&self, spec: Filter) -> Result<Vec<Task>, DbError> {
        let results = match spec {
            Filter::Status(status) => {
                query_as!(
                    Task,
                    r#"
                SELECT id, status as "status: TaskStatus", kind as "kind: TaskKind", process_at
                FROM tasks
                WHERE status = $1
                ORDER BY process_at
                "#,
                    status as TaskStatus
                )
                .fetch_all(&self.pool)
                .await?
            }
            Filter::Kind(kind) => {
                query_as!(
                    Task,
                    r#"
                SELECT id, status as "status: TaskStatus", kind as "kind: TaskKind", process_at
                FROM tasks
                WHERE kind = $1
                ORDER BY process_at
                "#,
                    kind as TaskKind
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(results)
    }

    pub async fn get_task(&self, id: &Uuid) -> Result<Option<Task>, DbError> {
        let result = query_as!(
            Task,
            r#"
            SELECT id, status as "status: TaskStatus", kind as "kind: TaskKind", process_at
            FROM tasks
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    pub async fn delete_task(&self, id: &Uuid) -> Result<(), DbError> {
        query!("DELETE FROM tasks WHERE id = $1", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // fn start_processing(&self, con: &mut redis::Connection, task_id: &str) -> Result<(), DbError> {
    //     todo!()
    // }

    // fn with_transaction<T, F: FnOnce(&mut redis::Connection) -> Result<T, DbError>>(
    //     &self,
    //     con: &mut redis::Connection,
    //     body: F,
    // ) -> Result<T, DbError> {
    //     let _: () = redis::cmd("MULTI").query(con)?;
    //     match body(con) {
    //         Ok(result) => {
    //             let _: () = redis::cmd("EXEC").query(con)?;
    //             Ok(result)
    //         }
    //         Err(e) => {
    //             let _: () = redis::cmd("DISCARD").query(con)?;
    //             Err(e)
    //         }
    //     }
    // }
}

#[cfg(all(test, feature = "real_database_tests"))]
mod tests {
    use crate::TaskKind::Bar;

    use super::*;

    const DATABASE_URL: &str = env!("DATABASE_URL");

    #[tokio::test]
    async fn test_happy_path_single_task() {
        println!("Running test_happy_path_single_task");
        let now = Utc.with_ymd_and_hms(2022, 1, 2, 13, 14, 15).unwrap();
        let db = Db::try_new(DATABASE_URL)
            .await
            .expect("Unable to connect to redis");
        db.clear_database()
            .await
            .expect("Unable to clear the database");
        let task = Task::with_current_time(Bar, now);
        db.add_task(&task)
            .await
            .expect("Unable to add task to the database");

        let next_task = db
            .get_next_task_executable_at(Utc::now())
            .await
            .expect("Unable to get next task");

        assert_eq!(next_task.map(|t| t.id), Some(task.id));
    }

    #[tokio::test]
    async fn test_queue_returns_earliest_task() {
        println!("Running test_queue_returns_earliest_task");
        let now = Utc.with_ymd_and_hms(2022, 1, 2, 13, 14, 15).unwrap();
        let db = Db::try_new(DATABASE_URL)
            .await
            .expect("Unable to connect to redis");
        db.clear_database()
            .await
            .expect("Unable to clear the database");
        let task1 = Task::with_current_time(TaskKind::Foo, now);
        let task2 = Task::with_current_time(TaskKind::Bar, now);
        db.add_task(&task1)
            .await
            .expect("Unable to add task to the database");
        db.add_task(&task2)
            .await
            .expect("Unable to add task to the database");

        let next_task = db
            .get_next_task_executable_at(Utc::now())
            .await
            .expect("Unable to get next task");

        assert_eq!(next_task.map(|t| t.id), Some(task2.id));
    }

    #[tokio::test]
    async fn test_queue_can_remove_task() {
        let now = Utc.with_ymd_and_hms(2022, 1, 2, 13, 14, 15).unwrap();
        let db = Db::try_new(DATABASE_URL)
            .await
            .expect("Unable to connect to redis");
        db.clear_database()
            .await
            .expect("Unable to clear the database");
        let task1 = Task::with_current_time(TaskKind::Foo, now);
        let task2 = Task::with_current_time(TaskKind::Bar, now);
        db.add_task(&task1)
            .await
            .expect("Unable to add task to the database");
        db.add_task(&task2)
            .await
            .expect("Unable to add task to the database");

        let tasks_before = db.get_tasks().await.expect("Unable to get tasks");

        let next_task = db
            .get_next_task_executable_at(Utc::now())
            .await
            .expect("Unable to get next task");

        assert_eq!(next_task.map(|t| t.id), Some(task2.id));

        db.complete_task(task2.id)
            .await
            .expect("Unable to complete task");

        let tasks_after = db.get_tasks().await.expect("Unable to get tasks");

        println!("Tasks before: {:?}", tasks_before);
        println!("Tasks after: {:?}", tasks_after);

        let next_task = db
            .get_next_task_executable_at(Utc::now())
            .await
            .expect("Unable to get next task");

        assert_eq!(next_task.map(|t| t.id), Some(task1.id));
    }
}
