use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{DbError, TaskKind, TaskStatus};

#[derive(Debug, Serialize, Deserialize)]
pub enum Filter {
    Status(TaskStatus),
    Kind(TaskKind),
}

impl FromStr for Filter {
    // Future Improvement: Add a better error type
    type Err = DbError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        match parts.as_slice() {
            ["status", status] => {
                let status = TaskStatus::from_str(status)?;
                Ok(Self::Status(status))
            }
            ["kind", kind] => {
                let kind = TaskKind::from_str(kind)?;
                Ok(Self::Kind(kind))
            }
            _ => Err(DbError::InvalidFilter(s.to_string())),
        }
    }
}
