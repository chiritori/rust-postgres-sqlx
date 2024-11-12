use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::{
    model::{NoteModel, NoteModelResponse},
    schema::{CreateNoteSchema, FilterOptions, UpdateNoteSchema},
    AppState,
};

pub async fn health_check_handler() -> impl IntoResponse {
    const MESSAGE: &str = "API Services";

    let json_response = serde_json::json!({
        "status": "ok",
        "message": MESSAGE
    });

    Json(json_response)
}

pub async fn note_list_handler(
    opts: Option<Query<FilterOptions>>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Param
    let Query(opts) = opts.unwrap_or_default();

    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    // Query with macro
    let notes = sqlx::query_as!(
        NoteModel,
        "SELECT * FROM notes ORDER by id LIMIT $1 OFFSET $2",
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await
    .map_err(|e| {
        let error_response = serde_json::json!({
            "status": "error",
            "message": format!("Database error: { }", e),
        });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response))
    })?;

    // Response
    let note_responses = notes
        .iter()
        .map(|note| to_note_response(&note))
        .collect::<Vec<NoteModelResponse>>();

    let json_response = serde_json::json!({
        "status": "ok",
        "count": note_responses.len(),
        "notes": note_responses
    });

    Ok(Json(json_response))
}

pub async fn create_note_handler(
    State(data): State<Arc<AppState>>,
    Json(body): Json<CreateNoteSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Insert
    let query_result = sqlx::query_as!(
        NoteModel,
        "INSERT INTO notes (title,content) VALUES ($1, $2) RETURNING *",
        body.title.to_string(),
        body.content.to_string(),
    )
    .fetch_one(&data.db)
    .await;
    match query_result {
        Ok(note) => {
            let note_response = serde_json::json!({
                "status": "success",
                "data": serde_json::json!({
                  "note": note
                })
            });
            return Ok(Json(note_response));
        }
        Err(e) => {
            if e.to_string()
                .contains("duplicate key value violates unique constraint")
            {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({"status": "fail","message": "Note with that title already exists"})),
                ));
            }

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": format!("{:?}", e)})),
            ));
        }
    }
}

pub async fn get_note_handler(
    Path(id): Path<uuid::Uuid>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // get not using query macro
    let query_result = sqlx::query_as!(NoteModel, "SELECT * FROM notes WHERE id = $1", id)
        .fetch_one(&data.db)
        .await;

    // check & response
    match query_result {
        Ok(note) => {
            let note_response = serde_json::json!({
                "status": "success",
                "data": serde_json::json!({
                    "note": to_note_response(&note)
                })
            });

            return Ok(Json(note_response));
        }
        Err(sqlx::Error::RowNotFound) => {
            let error_response = serde_json::json!({
                "status": "fail",
                "message": format!("Note with ID: {} not found", id)
            });
            return Err((StatusCode::NOT_FOUND, Json(error_response)));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": format!("{:?}", e)})),
            ));
        }
    };
}

pub async fn edit_note_handler(
    Path(id): Path<uuid::Uuid>,
    State(data): State<Arc<AppState>>,
    Json(body): Json<UpdateNoteSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let query_result = sqlx::query_as!(NoteModel, "SELECT * FROM notes WHERE id = $1", id)
        .fetch_one(&data.db)
        .await;

    if query_result.is_err() {
        let error_response = serde_json::json!({
            "status": "fail",
            "message": format!("Note with ID: {} not found", id)
        });
        return Err((StatusCode::NOT_FOUND, Json(error_response)));
    }

    let now = chrono::Utc::now();
    let note = query_result.unwrap();

    let query_result = sqlx::query_as!(
        NoteModel,
        "UPDATE notes SET title = $1, content = $2, is_published = $3, updated_at = $4 WHERE id = $5 RETURNING *",
        body.title.to_owned().unwrap_or(note.title),
        body.content.to_owned().unwrap_or(note.content),
        body.is_published.unwrap_or(note.is_published),
        now,
        id
    )
    .fetch_one(&data.db)
    .await
    ;

    match query_result {
        Ok(note) => {
            let note_response = serde_json::json!({"status": "success","data": serde_json::json!({
                "note": note
            })});

            return Ok(Json(note_response));
        }
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": format!("{:?}", err)})),
            ));
        }
    }
}
/*
pub async fn edit_note_handler(
    Path(id): Path<uuid::Uuid>,
    State(data): State<Arc<AppState>>,
    Json(body): Json<UpdateNoteSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // validate note without query macro
    let query_result = sqlx::query_as!(NoteModel, "SELECT * FROM notes WHERE id = $1", id)
        .fetch_one(&data.db)
        .await;

    // fetch the result
    let note = match query_result {
        Ok(note) => note,
        Err(sqlx::Error::RowNotFound) => {
            let error_response = serde_json::json!({
                "status": "error",
                "message": format!("Note with ID: {} not found", id)
            });
            return Err((StatusCode::NOT_FOUND, Json(error_response)));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("{:?}", e)
                })),
            ));
        }
    };

    // parse data
    let is_published = body.is_published.unwrap_or(note.is_published != 0);
    let i8_is_published = is_published as i8;
    let now = Utc::now();
    let note = query_result.unwrap();

    // Update (if empty, use old value)
    let update_result =
    sqlx::query_as!(
        NoteModel,
        "UPDATE notes SET title = $1, content = $2, is_published = $3, updated_at = $4 WHERE id = $5 RETURNING *",
        body.title.to_owned().unwrap_or(note.title),
        body.content.to_owned().unwrap_or(note.content),
        i8_is_published,
        now,
        id
    )
    .fetch_one(&data.db)
    .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "status": "error",
                        "message": format!("{:?}", e)
                    })),
                )
            })?;

    // if no data affected (or deleted when wanted to update)
    if update_result.rows_affected() == 0 {
        let error_response = serde_json::json!({
            "status": "error",
            "message": format!("Note with ID: {} not found", id)
        });
        return Err((StatusCode::NOT_FOUND, Json(error_response)));
    }

    // // get updated data with query macro
    // let updated_note = sqlx::query_as!(
    //     NoteModel,
    //     r#"SELECT * FROM notes WHERE id = ?"#,
    //     &id
    // )
    // .fetch_one(&data.db)
    // .await
    // .map_err(|e| {
    //     (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         Json(json!({"status": "error","message": format!("{:?}", e)})),
    //     )
    // })?;

    // get updated data without query macro
    let updated_note = sqlx::query_as::<_, NoteModel>(r#"SELECT * FROM notes WHERE id = ?"#)
        .bind(&id)
        .fetch_one(&data.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": format!("{:?}", e)})),
            )
        })?;

    let note_response = serde_json::json!({
        "status": "success",
        "data": serde_json::json!({
            "note": to_note_response(&updated_note)
        })
    });

    Ok(Json(note_response))
}
*/
pub async fn delete_note_handler(
    Path(id): Path<String>,
    State(data): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // // delete with query macro
    // let query_result = sqlx::query!(r#"DELETE FROM notes WHERE id = ?"#, &id)
    //     .execute(&data.db)
    //     .await
    //     .map_err(|e| {
    //         (
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             Json(json!({
    //                 "status": "error",
    //                 "message": format!("{:?}", e)
    //             })),
    //         )
    //     })?;

    // delete not using query macro
    let query_result = sqlx::query(r#"DELETE FROM notes WHERE id = ?"#)
        .bind(&id)
        .execute(&data.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error","message": format!("{:?}", e)})),
            )
        })?;

    // response
    if query_result.rows_affected() == 0 {
        let error_response = serde_json::json!({
            "status": "error",
            "message": format!("Note with ID: {} not found", id)
        });
        return Err((StatusCode::NOT_FOUND, Json(error_response)));
    }

    Ok(StatusCode::OK)
}

// Convert DB Model to Response
fn to_note_response(note: &NoteModel) -> NoteModelResponse {
    NoteModelResponse {
        id: note.id.to_owned(),
        title: note.title.to_owned(),
        content: note.content.to_owned(),
        is_published: note.is_published.to_owned(),
        created_at: note.created_at.unwrap(),
        updated_at: note.updated_at.unwrap(),
    }
}
