use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Extension, Json, Router,
};
use common::{Db, DbError, Task};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use types::{FilterParam, TaskCreateResponse, TaskRequest};
use uuid::Uuid;

mod types;

#[axum::debug_handler]
async fn create_task(
    db: Extension<Db>,
    Json(request): Json<TaskRequest>,
) -> Result<Json<TaskCreateResponse>, impl IntoResponse> {
    let task: Task = request.into();
    let result = db.add_task(&task).await;
    match result {
        Ok(_) => Ok(Json(TaskCreateResponse { id: task.id })),
        Err(_) => Err((StatusCode::BAD_REQUEST, "Failed to create task")),
    }
}

async fn get_task_list(
    db: Extension<Db>,
    Query(filters): Query<FilterParam>,
) -> Result<Json<Vec<Task>>, impl IntoResponse> {
    let result = match filters.filter {
        Some(filter) => db.get_filtered_tasks(filter).await,
        None => db.get_tasks().await,
    };
    match result {
        Ok(tasks) => Ok(Json(tasks)),
        Err(err) => match err {
            DbError::InvalidFilter(_) => Err((StatusCode::BAD_REQUEST, "Invalid filter")),
            DbError::InvalidStatus(_) => Err((StatusCode::BAD_REQUEST, "Invalid status")),
            DbError::InvalidKind(_) => Err((StatusCode::BAD_REQUEST, "Invalid kind")),
            _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get task list")),
        },
    }
}

async fn get_task(
    db: Extension<Db>,
    Path(id): Path<Uuid>,
) -> Result<Json<Task>, impl IntoResponse> {
    let result = db.get_task(&id).await;
    match result {
        Ok(Some(task)) => Ok(Json(task)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Task not found")),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get task")),
    }
}

async fn delete_task(db: Extension<Db>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let result = db.delete_task(&id).await;
    match result {
        Ok(_) => (StatusCode::NO_CONTENT, "Deleted"),
        // Future Improvement: Include the error in the response
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete task"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = Db::try_new(&database_url)
        .await
        .expect("Unable to connect to postgres");
    let db_layer = ServiceBuilder::new().layer(Extension(db));
    let app = Router::new()
        .route("/tasks", put(create_task).get(get_task_list))
        .route("/tasks/:id", get(get_task).delete(delete_task))
        // We use an extension layer because we want to only have a single version of the database, and this is
        // the least verbose way to do it.
        .layer(db_layer);
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
