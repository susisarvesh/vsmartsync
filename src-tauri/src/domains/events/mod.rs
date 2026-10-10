//! Device access history.
//!
//! The door decides entry. This module stores the events the device already
//! reported and shows them later. It does not grant access.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::common::{DevicePasswordVault, SecretError};
use crate::database::models::AccessEventWrite;
use crate::database::repositories::{
    AccessEventRepository, DeviceRepository, DeviceUserRepository, UserRepository,
};
use crate::database::{DatabaseError, DatabaseRuntime, DbPool};
use crate::domains::devices::DeviceStatus;
use crate::domains::enrollments::DeviceEnrollmentGate;
use crate::matrix::events::{user_event_ids_with_field1_user, EventCount};
use crate::matrix::{MatrixAdapter, MatrixAdapterError};

const MAX_BATCHES_PER_FETCH: u32 = 20;

#[derive(Debug, Error)]
pub enum EventError {
    #[error("DATABASE_UNAVAILABLE")]
    Unavailable,
    #[error("DATABASE_ERROR")]
    Database,
    #[error("DEVICE_NOT_FOUND")]
    DeviceNotFound,
    #[error("DEVICE_INACTIVE")]
    DeviceInactive,
    #[error("DEVICE_BUSY")]
    DeviceBusy,
    #[error("DEVICE_OFFLINE")]
    Offline,
    #[error("MATRIX_TIMEOUT")]
    Timeout,
    #[error("MATRIX_UNREACHABLE")]
    Unreachable,
    #[error("MATRIX_AUTH_FAILED")]
    AuthFailed,
    #[error("MATRIX_BAD_RESPONSE")]
    BadResponse,
    #[error("SECRET_UNAVAILABLE")]
    SecretUnavailable,
}

/// Position already stored for a device, or the device's current count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventCursor {
    pub roll_over_count: i64,
    pub seq_number: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchStep {
    /// First contact. Remember the current count and do not import older events.
    Anchor(EventCursor),
    Stop,
    Request(EventCursor),
}

pub fn fetch_step(saved: Option<EventCursor>, current: EventCount) -> FetchStep {
    let current = EventCursor {
        roll_over_count: current.roll_over_count,
        seq_number: current.seq_number,
    };
    match saved {
        None => FetchStep::Anchor(current),
        Some(cursor) if caught_up(cursor, current) => FetchStep::Stop,
        Some(cursor) => FetchStep::Request(next_seq(cursor)),
    }
}

pub fn caught_up(cursor: EventCursor, current: EventCursor) -> bool {
    (cursor.roll_over_count, cursor.seq_number) >= (current.roll_over_count, current.seq_number)
}

pub fn next_seq(cursor: EventCursor) -> EventCursor {
    EventCursor {
        roll_over_count: cursor.roll_over_count,
        seq_number: cursor.seq_number.saturating_add(1),
    }
}

pub fn advance_cursor(cursor: EventCursor, roll_over_count: i64, seq_number: i64) -> EventCursor {
    if (roll_over_count, seq_number) > (cursor.roll_over_count, cursor.seq_number) {
        EventCursor {
            roll_over_count,
            seq_number,
        }
    } else {
        cursor
    }
}

/// When a batch is empty and the device rollover moved forward, continue on the new rollover.
pub fn after_empty_batch(cursor: EventCursor, current: EventCursor) -> Option<EventCursor> {
    if current.roll_over_count > cursor.roll_over_count {
        Some(EventCursor {
            roll_over_count: current.roll_over_count,
            seq_number: 0,
        })
    } else {
        None
    }
}

/// One stored access event. The name and device name are the values saved at ingest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessEvent {
    pub id: Uuid,
    pub device_id: Uuid,
    pub device_name: String,
    pub occurred_label: String,
    pub device_date: String,
    pub device_time: String,
    pub person_name: Option<String>,
    pub event_id: Option<String>,
    pub details: String,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchDeviceEventsResult {
    pub device_id: Uuid,
    pub imported: u32,
    pub anchored: bool,
}

pub async fn list_access_events(
    events: &AccessEventRepository,
    device_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<AccessEvent>, EventError> {
    let limit = limit.unwrap_or(crate::database::repositories::ACCESS_EVENT_LIST_LIMIT);
    events
        .fill_missing_person_names(device_id, &user_event_ids_with_field1_user())
        .await
        .map_err(map_db_error)?;
    let rows = events.list(device_id, limit).await.map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .map(|row| AccessEvent {
            id: row.id,
            device_id: row.device_id,
            device_name: row.device_name,
            occurred_label: occurred_label(&row.device_date, &row.device_time),
            device_date: row.device_date,
            device_time: row.device_time,
            person_name: row.person_name,
            event_id: row.event_id,
            details: detail_label(&[
                row.detail_1,
                row.detail_2,
                row.detail_3,
                row.detail_4,
                row.detail_5,
            ]),
            ingested_at: row.ingested_at,
        })
        .collect())
}

pub async fn fetch_device_events(
    devices: &DeviceRepository,
    events: &AccessEventRepository,
    device_users: &DeviceUserRepository,
    users: &UserRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    gate: &DeviceEnrollmentGate,
    device_id: Uuid,
) -> Result<FetchDeviceEventsResult, EventError> {
    if gate.is_busy(device_id) {
        return Err(EventError::DeviceBusy);
    }
    let device = devices
        .find_by_id(device_id)
        .await
        .map_err(map_db_error)?
        .ok_or(EventError::DeviceNotFound)?;
    if DeviceStatus::parse(&device.status).map_err(|_| EventError::Unavailable)?
        != DeviceStatus::Active
    {
        return Err(EventError::DeviceInactive);
    }
    let password = vault
        .decrypt(&device.password_ciphertext)
        .map_err(map_secret_error)?;
    let port = u16::try_from(device.port).map_err(|_| EventError::BadResponse)?;
    let current = matrix
        .event_count(&device.host, port, &device.username, &password)
        .await
        .map_err(map_matrix_error)?;
    let saved = events
        .cursor(device_id)
        .await
        .map_err(map_db_error)?
        .map(|row| EventCursor {
            roll_over_count: i64::from(row.roll_over_count),
            seq_number: row.seq_number,
        });
    let now = Utc::now();
    match fetch_step(saved, current) {
        FetchStep::Anchor(cursor) => {
            save_cursor(events, device_id, cursor, now).await?;
            tracing::info!(
                device_id = %device_id,
                roll_over_count = cursor.roll_over_count,
                seq_number = cursor.seq_number,
                "access event cursor anchored"
            );
            return Ok(FetchDeviceEventsResult {
                device_id,
                imported: 0,
                anchored: true,
            });
        }
        FetchStep::Stop => {
            return Ok(FetchDeviceEventsResult {
                device_id,
                imported: 0,
                anchored: false,
            });
        }
        FetchStep::Request(_) => {}
    }

    let current_cursor = EventCursor {
        roll_over_count: current.roll_over_count,
        seq_number: current.seq_number,
    };
    let mut cursor = saved.expect("request step has a cursor");
    let mut imported = 0_u32;
    for _ in 0..MAX_BATCHES_PER_FETCH {
        if caught_up(cursor, current_cursor) {
            break;
        }
        let request = next_seq(cursor);
        let batch = matrix
            .device_events(
                &device.host,
                port,
                &device.username,
                &password,
                request.roll_over_count,
                request.seq_number,
            )
            .await
            .map_err(map_matrix_error)?;
        if batch.is_empty() {
            if let Some(moved) = after_empty_batch(cursor, current_cursor) {
                cursor = moved;
                save_cursor(events, device_id, cursor, Utc::now()).await?;
                continue;
            }
            break;
        }
        for event in batch {
            let (user_id, person_name) =
                person_for_ref(device_users, users, device_id, event.ref_user_id).await?;
            let roll_over_count =
                i32::try_from(event.roll_over_count).map_err(|_| EventError::BadResponse)?;
            let write = AccessEventWrite {
                id: Uuid::new_v4(),
                device_id,
                device_name: device.device_name.clone(),
                roll_over_count,
                seq_number: event.seq_number,
                event_id: event.event_id,
                device_date: event.device_date,
                device_time: event.device_time,
                user_id,
                person_name,
                ref_user_id: event.ref_user_id,
                detail_1: event.detail_1,
                detail_2: event.detail_2,
                detail_3: event.detail_3,
                detail_4: event.detail_4,
                detail_5: event.detail_5,
                ingested_at: Utc::now(),
            };
            if events.insert_ignore(&write).await.map_err(map_db_error)? {
                imported = imported.saturating_add(1);
            }
            cursor = advance_cursor(cursor, event.roll_over_count, event.seq_number);
        }
        save_cursor(events, device_id, cursor, Utc::now()).await?;
    }
    drop(password);
    tracing::info!(
        device_id = %device_id,
        imported,
        roll_over_count = cursor.roll_over_count,
        seq_number = cursor.seq_number,
        "access events fetched"
    );
    Ok(FetchDeviceEventsResult {
        device_id,
        imported,
        anchored: false,
    })
}

pub async fn poll_active_devices(runtime: &DatabaseRuntime, gate: &DeviceEnrollmentGate) {
    let Some(pool) = runtime.pool() else {
        return;
    };
    let Ok(matrix) = MatrixAdapter::new() else {
        tracing::warn!("access event poll skipped because the matrix client could not start");
        return;
    };
    let Ok(vault) = DevicePasswordVault::open() else {
        tracing::warn!(
            "access event poll skipped because the device password vault is unavailable"
        );
        return;
    };
    let devices = DeviceRepository::new(pool.clone());
    let listed = match devices.list().await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(error = %error, "access event poll could not list devices");
            return;
        }
    };
    for device in listed {
        if device.status != DeviceStatus::Active.as_str() {
            continue;
        }
        if gate.is_busy(device.id) {
            tracing::info!(
                device_id = %device.id,
                "skipped access event fetch during enrollment"
            );
            continue;
        }
        if let Err(error) = fetch_one(&pool, &vault, &matrix, gate, device.id).await {
            tracing::warn!(
                device_id = %device.id,
                error = %error,
                "access event fetch failed"
            );
        }
    }
}

async fn fetch_one(
    pool: &DbPool,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    gate: &DeviceEnrollmentGate,
    device_id: Uuid,
) -> Result<FetchDeviceEventsResult, EventError> {
    fetch_device_events(
        &DeviceRepository::new(pool.clone()),
        &AccessEventRepository::new(pool.clone()),
        &DeviceUserRepository::new(pool.clone()),
        &UserRepository::new(pool.clone()),
        vault,
        matrix,
        gate,
        device_id,
    )
    .await
}

async fn person_for_ref(
    device_users: &DeviceUserRepository,
    users: &UserRepository,
    device_id: Uuid,
    ref_user_id: Option<i64>,
) -> Result<(Option<Uuid>, Option<String>), EventError> {
    let Some(ref_user_id) = ref_user_id else {
        return Ok((None, None));
    };
    let Some(mapping) = device_users
        .find_by_device_and_ref(device_id, ref_user_id)
        .await
        .map_err(map_db_error)?
    else {
        return Ok((None, None));
    };
    let person_name = users
        .find_by_id(mapping.user_id)
        .await
        .map_err(map_db_error)?
        .map(|user| user.username);
    Ok((Some(mapping.user_id), person_name))
}

async fn save_cursor(
    events: &AccessEventRepository,
    device_id: Uuid,
    cursor: EventCursor,
    updated_at: DateTime<Utc>,
) -> Result<(), EventError> {
    let roll_over_count =
        i32::try_from(cursor.roll_over_count).map_err(|_| EventError::BadResponse)?;
    events
        .save_cursor(device_id, roll_over_count, cursor.seq_number, updated_at)
        .await
        .map_err(map_db_error)
}

pub fn occurred_label(date: &str, time: &str) -> String {
    let date_label = format_device_date(date);
    let time_label = format_device_time(time);
    match (date_label, time_label) {
        (Some(date), Some(time)) => format!("{date} {time}"),
        (Some(date), None) if time.is_empty() => date,
        (None, Some(time)) if date.is_empty() => time,
        _ => {
            let joined = format!("{date} {time}").trim().to_string();
            if joined.is_empty() {
                "Not reported".to_string()
            } else {
                joined
            }
        }
    }
}

fn format_device_date(value: &str) -> Option<String> {
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.len() == 8 {
        let day: u32 = digits[0..2].parse().ok()?;
        let month: u32 = digits[2..4].parse().ok()?;
        let year: i32 = digits[4..8].parse().ok()?;
        let name = month_name(month)?;
        if (1..=31).contains(&day) {
            return Some(format!("{day} {name} {year}"));
        }
    }
    None
}

fn format_device_time(value: &str) -> Option<String> {
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.len() == 6 {
        let hour: u32 = digits[0..2].parse().ok()?;
        let minute: u32 = digits[2..4].parse().ok()?;
        let second: u32 = digits[4..6].parse().ok()?;
        if hour <= 23 && minute <= 59 && second <= 59 {
            return Some(format!("{hour:02}:{minute:02}:{second:02}"));
        }
    }
    None
}

fn month_name(month: u32) -> Option<&'static str> {
    Some(match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => return None,
    })
}

fn detail_label(details: &[Option<String>]) -> String {
    details
        .iter()
        .filter_map(|value| value.as_deref().filter(|text| !text.is_empty()))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn map_db_error(error: DatabaseError) -> EventError {
    tracing::error!(error = %error, "access event database error");
    match error {
        DatabaseError::Connect(_) => EventError::Unavailable,
        DatabaseError::Query(_) | DatabaseError::Migrate(_) => EventError::Database,
    }
}

fn map_secret_error(error: SecretError) -> EventError {
    tracing::error!(error = %error, "device secret vault failed");
    EventError::SecretUnavailable
}

fn map_matrix_error(error: MatrixAdapterError) -> EventError {
    match error {
        MatrixAdapterError::Timeout => EventError::Timeout,
        MatrixAdapterError::Unreachable | MatrixAdapterError::InvalidTarget => {
            EventError::Unreachable
        }
        MatrixAdapterError::AuthFailed => EventError::AuthFailed,
        MatrixAdapterError::BadResponse
        | MatrixAdapterError::ApiError { .. }
        | MatrixAdapterError::InvalidArgument => EventError::BadResponse,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        advance_cursor, after_empty_batch, caught_up, fetch_step, next_seq, occurred_label,
        EventCount, EventCursor, FetchStep,
    };

    #[test]
    fn first_fetch_anchors_at_the_current_count() {
        let current = EventCount {
            roll_over_count: 6,
            seq_number: 1140,
        };
        assert_eq!(
            fetch_step(None, current),
            FetchStep::Anchor(EventCursor {
                roll_over_count: 6,
                seq_number: 1140,
            })
        );
    }

    #[test]
    fn next_fetch_requests_the_following_sequence() {
        let saved = EventCursor {
            roll_over_count: 6,
            seq_number: 1140,
        };
        let current = EventCount {
            roll_over_count: 6,
            seq_number: 1144,
        };
        assert_eq!(
            fetch_step(Some(saved), current),
            FetchStep::Request(EventCursor {
                roll_over_count: 6,
                seq_number: 1141,
            })
        );
        assert!(caught_up(
            EventCursor {
                roll_over_count: 6,
                seq_number: 1144,
            },
            EventCursor {
                roll_over_count: 6,
                seq_number: 1144,
            }
        ));
        assert_eq!(next_seq(saved).seq_number, 1141);
        assert_eq!(
            advance_cursor(saved, 6, 1142),
            EventCursor {
                roll_over_count: 6,
                seq_number: 1142,
            }
        );
        assert_eq!(advance_cursor(saved, 6, 1130), saved);
    }

    #[test]
    fn empty_batch_on_a_new_rollover_moves_forward() {
        let cursor = EventCursor {
            roll_over_count: 6,
            seq_number: 500_000,
        };
        let current = EventCursor {
            roll_over_count: 7,
            seq_number: 3,
        };
        assert_eq!(
            after_empty_batch(cursor, current),
            Some(EventCursor {
                roll_over_count: 7,
                seq_number: 0,
            })
        );
        assert_eq!(
            after_empty_batch(
                cursor,
                EventCursor {
                    roll_over_count: 6,
                    seq_number: 500_000,
                }
            ),
            None
        );
    }

    #[test]
    fn device_clock_is_labeled_without_changing_the_stored_text() {
        assert_eq!(occurred_label("31122022", "103008"), "31 Dec 2022 10:30:08");
    }
}
