use crate::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde_json::{json, Value};

use super::types::{ActivityLogData, ActivityLogGetRequest, ActivityLogGetResponse};
use crate::api_error::ApiError;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub async fn log_retrieval(
    State(app_state): State<AppState>,
    Query(query_params): Query<ActivityLogGetRequest>,
) -> Result<Json<Value>, ApiError> {
    // println!("\nLog Retrieval: {:?}\n", query_params);

    // Add default date if no cursor is provided
    let cursor: String = match query_params.cursor {
        Some(cursor1) => match OffsetDateTime::parse(&cursor1, &Rfc3339) {
            Ok(cur) => cur.format(&Rfc3339).unwrap(),
            Err(_) => return Err(ApiError::InvalidRequest("Invalid cursor".to_string())),
        },

        None => {
            let now = OffsetDateTime::now_utc();
            now.format(&Rfc3339).unwrap()
        }
    };

    let limit = match query_params.limit {
        Some(l) => {
            if !(1..=100).contains(&l) {
                return Err(ApiError::InvalidRequest(
                    "Limit must be a number between 1 and 100".to_string(),
                ));
            }
            l
        }
        None => 10,
    };

    println!("Limit: {}", limit);
    let rows: Vec<ActivityLogData> = sqlx::query_as::<_, ActivityLogData>(
        r#"
        SELECT 
            wallet_address,
            from_token,
            to_token,
            amount_from,
            percentage,
            amount_to,
            TO_CHAR(created_at, 'YYYY-MM-DD"T"HH24:MI:SSZ') AS created_at 
        FROM transactions_log
        WHERE created_at < $1::TIMESTAMPTZ
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(cursor)
    .bind(limit)
    .fetch_all(&app_state.db.pool)
    .await
    .map_err(ApiError::DatabaseError)?;

    // Map results to the response data structure
    let mut response_data: ActivityLogGetResponse = ActivityLogGetResponse {
        transactions: rows
            .into_iter()
            .map(|row| ActivityLogData {
                wallet_address: row.wallet_address,
                from_token: row.from_token,
                to_token: row.to_token,
                percentage: row.percentage,
                amount_from: row.amount_from,
                amount_to: row.amount_to,
                created_at: row.created_at,
            })
            .collect(),
        next_cursor: None,
    };

    // Check if there are more transactions
    if response_data.transactions.len() == limit as usize {
        let last_transaction = response_data.transactions.last().unwrap();
        response_data.next_cursor = Some(last_transaction.created_at.clone());
    }

    Ok(Json(json!(response_data)))
}
