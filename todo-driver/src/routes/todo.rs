use crate::models::{CreateTodo, Db, GameParam, Todo, UpdateTodo};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::Query;
use uuid::Uuid;

//
pub async fn todo_list_handler(
    Query(params): Query<GameParam>,
    State(db): State<Db>,
) -> impl IntoResponse {
    let categories = params.categories;
    match categories {
        Some(val) => tracing::debug!("categories {:?}", val),
        None => tracing::debug!("not setting categories"),
    }

    let todos = db.read().unwrap();
    let todos = todos.values().cloned().collect::<Vec<_>>();
    Json(todos)
}

//
pub async fn create_todo_handler(
    State(db): State<Db>,
    Json(input): Json<CreateTodo>,
) -> impl IntoResponse {
    let todo = Todo {
        id: Uuid::new_v4(),
        text: input.text,
        completed: false,
    };

    db.write().unwrap().insert(todo.id, todo.clone());

    (StatusCode::CREATED, Json(todo))
}

//
pub async fn edit_todo_handler(
    Path(id): Path<Uuid>,
    State(db): State<Db>,
    Json(input): Json<UpdateTodo>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut todo = db
        .read()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(text) = input.text {
        todo.text = text;
    }

    if let Some(completed) = input.completed {
        todo.completed = completed;
    }

    db.write().unwrap().insert(todo.id, todo.clone());

    Ok(Json(todo))
}

//
pub async fn delete_todo_handler(Path(id): Path<Uuid>, State(db): State<Db>) -> impl IntoResponse {
    if db.write().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
