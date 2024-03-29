#![allow(unused)]

use std::{fmt::Display, str::FromStr};

use chrono::{DateTime, Utc};
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
