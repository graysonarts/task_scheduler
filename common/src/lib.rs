#![allow(unused)]

use std::{fmt::Display, str::FromStr};

use chrono::{DateTime, Utc};
use rand::Rng;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgTypeInfo, Postgres, Type, TypeInfo};
use uuid::Uuid;

pub mod db;
pub mod filter;
pub use db::{Db, DbError};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, sqlx::Type)]
#[sqlx(type_name = "task_status_type")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::InProgress => write!(f, "InProgress"),
            Self::Completed => write!(f, "Completed"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = DbError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(Self::Pending),
            "InProgress" => Ok(Self::InProgress),
            "Completed" => Ok(Self::Completed),
            _ => Err(DbError::InvalidStatus(s.to_string())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, sqlx::Type)]
#[sqlx(type_name = "task_type")]
pub enum TaskKind {
    Foo,
    Bar,
    Baz,
}

impl TaskKind {
    pub fn process_delay(&self) -> chrono::Duration {
        match self {
            Self::Foo => chrono::Duration::seconds(3),
            _ => chrono::Duration::seconds(0),
        }
    }
}

impl FromStr for TaskKind {
    type Err = DbError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Foo" => Ok(Self::Foo),
            "Bar" => Ok(Self::Bar),
            "Baz" => Ok(Self::Baz),
            _ => Err(DbError::InvalidKind(s.to_string())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub kind: TaskKind,
    pub process_at: DateTime<Utc>,
    pub status: TaskStatus,
}

impl Task {
    pub fn new(kind: TaskKind) -> Self {
        Self::with_current_time(kind, Utc::now())
    }

    pub fn with_current_time(kind: TaskKind, now: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            process_at: now + kind.process_delay(),
            kind,
            status: TaskStatus::Pending,
        }
    }

    pub async fn run(&self) {
        match self.kind {
            TaskKind::Foo => self.run_foo_task(),
            TaskKind::Bar => self.run_bar_task().await,
            TaskKind::Baz => self.run_baz_task(),
        }
    }

    fn run_foo_task(&self) {
        // We don't sleep here because we calculate the 3 second delay when creating the task
        // This allows us to not have to sleep in the task itself. See `TaskKind::process_delay`
        println!("Foo {}", self.id);
    }

    async fn run_bar_task(&self) {
        // We need to set all of these headers, otherwise the url will send a 400 bad request
        let client = reqwest::ClientBuilder::new()
            .user_agent("Mozilla/5.0 (Reqwests) Gecko/20100101 Firefox/124.0")
            .build()
            .unwrap();
        let request = client
            .get("https://www.whattimeisitrightnow.com/")
            .header("Accept", "text/html")
            .header("host", "www.whattimeisitrightnow.com")
            .build()
            .unwrap();
        let response = client.execute(request).await;
        let message = match response {
            Err(err) => {
                if err.is_status() {
                    format!("{}", err.status().unwrap()) // Because we are checking that it's
                } else {
                    format!("{}", err)
                }
            }
            Ok(response) => {
                format!("{}", response.status())
            }
        };
        // It was unclear if I should prefix it with Bar or not, but based on the explicit text of
        // the requirements, I would just print the status code.
        println!("{}", message);
    }

    fn run_baz_task(&self) {
        let mut rng = rand::thread_rng();
        let random_number = rng.gen_range(0..=343);
        println!("Baz {}", random_number);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_kind_process_delay() {
        assert_eq!(TaskKind::Foo.process_delay(), chrono::Duration::seconds(3));
        assert_eq!(TaskKind::Bar.process_delay(), chrono::Duration::seconds(0));
        assert_eq!(TaskKind::Baz.process_delay(), chrono::Duration::seconds(0));
    }

    #[test]
    fn test_task_new() {
        let task = Task::new(TaskKind::Foo);
        assert_eq!(task.kind, TaskKind::Foo);
    }

    #[test]
    fn foo_is_3_seconds_from_now() {
        let now = Utc::now();
        let task = Task::with_current_time(TaskKind::Foo, now);
        assert_eq!(task.process_at, now + chrono::Duration::seconds(3));
    }
}
