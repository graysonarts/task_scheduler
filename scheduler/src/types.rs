use chrono::prelude::*;
use std::{fmt, str::FromStr};

use common::{filter::Filter, TaskKind};
use serde::{de, Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct TaskRequest {
    pub kind: TaskKind,
    pub execute_at: DateTime<Utc>,
}

impl From<TaskRequest> for common::Task {
    fn from(request: TaskRequest) -> Self {
        common::Task::with_current_time(request.kind, request.execute_at)
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskCreateResponse {
    pub id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FilterParam {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub filter: Option<Filter>,
}

// From axum example: https://github.com/tokio-rs/axum/blob/main/examples/query-params-with-empty-strings/src/main.rs
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}
