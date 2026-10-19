//! Device access history.
//!
//! The door decides entry. This module stores the events the device already
//! reported and shows them later. It does not grant access.

mod schedule;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

pub use schedule::{PollStatus, PollTracker};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::common::{DevicePasswordVault, SecretError};
use crate::database::models::AccessEventWrite;
use crate::database::repositories::{
    AccessEventRepository, DeviceRepository, DeviceUserRepository,
};
use crate::database::{DatabaseError, DatabaseRuntime, DbPool};
use crate::domains::devices::DeviceStatus;
use crate::domains::enrollments::DeviceEnrollmentGate;
use crate::matrix::events::{user_event_ids_with_field1_user, EventCount};
use crate::matrix::{MatrixAdapter, MatrixAdapterError};

use schedule::{poll_wave, ReaderRef, VisitSample};

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
    /// Stored ciphertext does not decrypt with this workstation's key.
    #[error("SECRET_CORRUPT")]
    SecretCorrupt,
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
    pub roll_over_count: i64,
    pub seq_number: i64,
    /// Time to decrypt the stored device password. Zero when decrypt did not run.
    pub decrypt_ms: u64,
    /// Time in Matrix HTTP calls for this visit, including authentication.
    pub http_ms: u64,
    /// Time in PostgreSQL for this visit, including connection acquisition.
    pub database_ms: u64,
}

pub async fn list_access_events(
    events: &AccessEventRepository,
    device_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<AccessEvent>, EventError> {
    let limit = limit.unwrap_or(crate::database::repositories::ACCESS_EVENT_LIST_LIMIT);
    let mut rows = events.list(device_id, limit).await.map_err(map_db_error)?;
    if page_needs_person_names(&rows) {
        events
            .fill_missing_person_names(device_id, &user_event_ids_with_field1_user())
            .await
            .map_err(map_db_error)?;
        rows = events.list(device_id, limit).await.map_err(map_db_error)?;
    }
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

struct VisitTrace {
    device_id: Uuid,
    started: Instant,
    decrypt_ms: u128,
    http_ms: u128,
    database_ms: u128,
    outcome: String,
    imported: u32,
}

impl Drop for VisitTrace {
    fn drop(&mut self) {
        tracing::debug!(
            device_id = %self.device_id,
            outcome = %self.outcome,
            decrypt_ms = self.decrypt_ms,
            http_ms = self.http_ms,
            database_ms = self.database_ms,
            total_ms = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX),
            imported = self.imported,
            "reader visit"
        );
    }
}

impl VisitTrace {
    fn new(device_id: Uuid) -> Self {
        Self {
            device_id,
            started: Instant::now(),
            decrypt_ms: 0,
            http_ms: 0,
            database_ms: 0,
            outcome: "INTERRUPTED".to_string(),
            imported: 0,
        }
    }

    fn fail(&mut self, error: EventError) -> EventError {
        self.outcome = error.to_string();
        error
    }

    fn result(&self, anchored: bool, cursor: EventCursor) -> FetchDeviceEventsResult {
        FetchDeviceEventsResult {
            device_id: self.device_id,
            imported: self.imported,
            anchored,
            roll_over_count: cursor.roll_over_count,
            seq_number: cursor.seq_number,
            decrypt_ms: u64::try_from(self.decrypt_ms).unwrap_or(u64::MAX),
            http_ms: u64::try_from(self.http_ms).unwrap_or(u64::MAX),
            database_ms: u64::try_from(self.database_ms).unwrap_or(u64::MAX),
        }
    }
}

pub async fn fetch_device_events(
    devices: &DeviceRepository,
    events: &AccessEventRepository,
    device_users: &DeviceUserRepository,
    vault: &DevicePasswordVault,
    matrix: &MatrixAdapter,
    gate: &DeviceEnrollmentGate,
    device_id: Uuid,
) -> Result<FetchDeviceEventsResult, EventError> {
    let mut trace = VisitTrace::new(device_id);
    if gate.is_busy(device_id) {
        return Err(trace.fail(EventError::DeviceBusy));
    }
    let step = Instant::now();
    let device = match devices.find_by_id(device_id).await {
        Ok(Some(device)) => device,
        Ok(None) => return Err(trace.fail(EventError::DeviceNotFound)),
        Err(error) => return Err(trace.fail(map_db_error(error))),
    };
    trace.database_ms += step.elapsed().as_millis();
    let status = match DeviceStatus::parse(&device.status) {
        Ok(status) => status,
        Err(_) => return Err(trace.fail(EventError::Unavailable)),
    };
    if status != DeviceStatus::Active {
        return Err(trace.fail(EventError::DeviceInactive));
    }
    let step = Instant::now();
    let password = match vault.decrypt(&device.password_ciphertext) {
        Ok(password) => password,
        Err(error) => {
            trace.decrypt_ms += step.elapsed().as_millis();
            return Err(trace.fail(map_secret_error(error)));
        }
    };
    trace.decrypt_ms += step.elapsed().as_millis();
    let port = match u16::try_from(device.port) {
        Ok(port) => port,
        Err(_) => return Err(trace.fail(EventError::BadResponse)),
    };
    let step = Instant::now();
    let current = match matrix
        .event_count(&device.host, port, &device.username, &password)
        .await
    {
        Ok(current) => current,
        Err(error) => {
            trace.http_ms += step.elapsed().as_millis();
            return Err(trace.fail(map_matrix_error(error)));
        }
    };
    trace.http_ms += step.elapsed().as_millis();
    let step = Instant::now();
    let saved = match events.cursor(device_id).await {
        Ok(row) => row.map(|row| EventCursor {
            roll_over_count: i64::from(row.roll_over_count),
            seq_number: row.seq_number,
        }),
        Err(error) => {
            trace.database_ms += step.elapsed().as_millis();
            return Err(trace.fail(map_db_error(error)));
        }
    };
    trace.database_ms += step.elapsed().as_millis();
    let now = Utc::now();
    match fetch_step(saved, current) {
        FetchStep::Anchor(cursor) => {
            let step = Instant::now();
            if let Err(error) = save_cursor(events, device_id, cursor, now).await {
                trace.database_ms += step.elapsed().as_millis();
                return Err(trace.fail(error));
            }
            trace.database_ms += step.elapsed().as_millis();
            trace.outcome = "REACHED".to_string();
            tracing::info!(
                device_id = %device_id,
                roll_over_count = cursor.roll_over_count,
                seq_number = cursor.seq_number,
                "access event cursor anchored"
            );
            return Ok(trace.result(true, cursor));
        }
        FetchStep::Stop => {
            let cursor = saved.unwrap_or(EventCursor {
                roll_over_count: current.roll_over_count,
                seq_number: current.seq_number,
            });
            trace.outcome = "REACHED".to_string();
            return Ok(trace.result(false, cursor));
        }
        FetchStep::Request(_) => {}
    }

    let current_cursor = EventCursor {
        roll_over_count: current.roll_over_count,
        seq_number: current.seq_number,
    };
    let step = Instant::now();
    let names = match person_names(device_users, device_id).await {
        Ok(names) => names,
        Err(error) => {
            trace.database_ms += step.elapsed().as_millis();
            return Err(trace.fail(error));
        }
    };
    trace.database_ms += step.elapsed().as_millis();
    let mut cursor = saved.expect("request step has a cursor");
    for _ in 0..MAX_BATCHES_PER_FETCH {
        if caught_up(cursor, current_cursor) {
            break;
        }
        let request = next_seq(cursor);
        let step = Instant::now();
        let batch = match matrix
            .device_events(
                &device.host,
                port,
                &device.username,
                &password,
                request.roll_over_count,
                request.seq_number,
            )
            .await
        {
            Ok(batch) => batch,
            Err(error) => {
                trace.http_ms += step.elapsed().as_millis();
                return Err(trace.fail(map_matrix_error(error)));
            }
        };
        trace.http_ms += step.elapsed().as_millis();
        if batch.is_empty() {
            if let Some(moved) = after_empty_batch(cursor, current_cursor) {
                cursor = moved;
                let step = Instant::now();
                if let Err(error) = save_cursor(events, device_id, cursor, Utc::now()).await {
                    trace.database_ms += step.elapsed().as_millis();
                    return Err(trace.fail(error));
                }
                trace.database_ms += step.elapsed().as_millis();
                continue;
            }
            break;
        }
        let mut writes = Vec::with_capacity(batch.len());
        for event in batch {
            let (user_id, person_name) = person_from_names(&names, event.ref_user_id);
            let roll_over_count = match i32::try_from(event.roll_over_count) {
                Ok(value) => value,
                Err(_) => return Err(trace.fail(EventError::BadResponse)),
            };
            writes.push(AccessEventWrite {
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
            });
            cursor = advance_cursor(cursor, event.roll_over_count, event.seq_number);
        }
        let roll_over_count = match i32::try_from(cursor.roll_over_count) {
            Ok(value) => value,
            Err(_) => return Err(trace.fail(EventError::BadResponse)),
        };
        let step = Instant::now();
        let inserted = match events
            .persist_batch_and_cursor(
                &writes,
                device_id,
                roll_over_count,
                cursor.seq_number,
                Utc::now(),
            )
            .await
        {
            Ok(inserted) => inserted,
            Err(error) => {
                trace.database_ms += step.elapsed().as_millis();
                return Err(trace.fail(map_db_error(error)));
            }
        };
        trace.database_ms += step.elapsed().as_millis();
        trace.imported = trace
            .imported
            .saturating_add(u32::try_from(inserted).unwrap_or(u32::MAX));
    }
    drop(password);
    trace.outcome = "REACHED".to_string();
    tracing::info!(
        device_id = %device_id,
        imported = trace.imported,
        roll_over_count = cursor.roll_over_count,
        seq_number = cursor.seq_number,
        "access events fetched"
    );
    Ok(trace.result(false, cursor))
}

pub async fn poll_active_devices(
    runtime: &DatabaseRuntime,
    gate: &DeviceEnrollmentGate,
    matrix: &MatrixAdapter,
    tracker: &PollTracker,
) {
    if !tracker.try_begin_cycle() {
        tracing::warn!("access event poll skipped because the previous cycle is still running");
        return;
    }
    let started = Instant::now();
    let summary = run_poll_cycle(runtime, gate, matrix, tracker).await;
    record_pool(runtime, tracker);
    tracker.finish_cycle(
        started.elapsed(),
        summary.configured,
        summary.due,
        summary.cooling,
    );
    let status = tracker.snapshot();
    tracing::info!(
        configured = summary.configured,
        due = summary.due,
        cooling = summary.cooling,
        reached = status.last_cycle_reached,
        failed = status.last_cycle_failed,
        events = status.last_cycle_events,
        visit_p50_ms = status.visit_p50_ms,
        visit_p95_ms = status.visit_p95_ms,
        elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        concurrency = tracker.config().concurrency,
        "access event poll cycle finished"
    );
}

struct CycleSummary {
    configured: usize,
    due: usize,
    cooling: usize,
}

async fn run_poll_cycle(
    runtime: &DatabaseRuntime,
    gate: &DeviceEnrollmentGate,
    matrix: &MatrixAdapter,
    tracker: &PollTracker,
) -> CycleSummary {
    let Some(pool) = runtime.pool() else {
        tracing::warn!("access event poll skipped because postgresql is unavailable");
        return CycleSummary {
            configured: 0,
            due: 0,
            cooling: 0,
        };
    };
    let vault = match DevicePasswordVault::open() {
        Ok(vault) => vault,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "access event poll skipped because the device password vault could not be opened"
            );
            return CycleSummary {
                configured: 0,
                due: 0,
                cooling: 0,
            };
        }
    };
    let devices = DeviceRepository::new(pool.clone());
    let listed = match devices.list_for_poll().await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(error = %error, "access event poll could not list devices");
            return CycleSummary {
                configured: 0,
                due: 0,
                cooling: 0,
            };
        }
    };
    let mut readers = Vec::new();
    let mut busy = HashSet::new();
    for device in listed {
        if device.status != DeviceStatus::Active.as_str() {
            continue;
        }
        if gate.is_busy(device.id) {
            busy.insert(device.id);
            tracing::info!(
                device_id = %device.id,
                "skipped access event fetch during enrollment"
            );
        }
        readers.push(ReaderRef {
            id: device.id,
            name: device.device_name,
        });
    }
    let configured = readers.len();
    let listed_ids = readers.iter().map(|reader| reader.id).collect();
    tracker.retain_listed(&listed_ids);
    let now = Utc::now();
    let due = tracker.select_due(now, &readers, &busy);
    let due_count = due.len();
    let cooling = configured
        .saturating_sub(busy.len())
        .saturating_sub(due_count);
    let concurrency = tracker.config().concurrency;
    let vault = Arc::new(vault);
    let pool = pool.clone();
    let matrix = matrix.clone();
    let gate = gate.clone();
    let tracker_for_wave = tracker.clone();
    let (results, _workers) = poll_wave(due, concurrency, move |device_id| {
        let pool = pool.clone();
        let vault = Arc::clone(&vault);
        let matrix = matrix.clone();
        let gate = gate.clone();
        let tracker = tracker_for_wave.clone();
        async move {
            if !tracker.try_claim(device_id) {
                return Err(EventError::DeviceBusy.to_string());
            }
            let result = fetch_one(&pool, vault.as_ref(), &matrix, &gate, device_id).await;
            tracker.release(device_id);
            result.map_err(|error| error.to_string())
        }
    })
    .await;
    let recorded_at = Utc::now();
    let mut visits = Vec::with_capacity(results.len());
    for polled in results {
        let sample = match &polled.output {
            Ok(fetched) => {
                tracker.record_success(polled.id, fetched.imported, recorded_at);
                tracker.record_checkpoint(
                    polled.id,
                    fetched.roll_over_count,
                    fetched.seq_number,
                    recorded_at,
                );
                VisitSample {
                    reached: true,
                    code: None,
                    imported: fetched.imported,
                    queue_ms: polled.queue_ms,
                    total_ms: polled.visit_ms,
                    decrypt_ms: Some(fetched.decrypt_ms),
                    http_ms: Some(fetched.http_ms),
                    database_ms: Some(fetched.database_ms),
                }
            }
            Err(code) if code == "DEVICE_BUSY" || code == "DEVICE_INACTIVE" => {
                tracing::info!(
                    device_id = %polled.id,
                    error = %code,
                    "access event fetch skipped"
                );
                VisitSample {
                    reached: false,
                    code: Some(code.clone()),
                    imported: 0,
                    queue_ms: polled.queue_ms,
                    total_ms: polled.visit_ms,
                    decrypt_ms: None,
                    http_ms: None,
                    database_ms: None,
                }
            }
            Err(code) => {
                tracker.record_failure(polled.id, code, recorded_at);
                tracing::warn!(
                    device_id = %polled.id,
                    error = %code,
                    "access event fetch failed"
                );
                VisitSample {
                    reached: false,
                    code: Some(code.clone()),
                    imported: 0,
                    queue_ms: polled.queue_ms,
                    total_ms: polled.visit_ms,
                    decrypt_ms: None,
                    http_ms: None,
                    database_ms: None,
                }
            }
        };
        visits.push(sample);
    }
    tracker.record_visits(&visits);
    CycleSummary {
        configured,
        due: due_count,
        cooling,
    }
}

fn record_pool(runtime: &DatabaseRuntime, tracker: &PollTracker) {
    let Some(pool) = runtime.pool() else {
        return;
    };
    let idle = u32::try_from(pool.num_idle()).unwrap_or(u32::MAX);
    let in_use = pool.size().saturating_sub(idle);
    tracker.set_pool(in_use, idle);
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
        vault,
        matrix,
        gate,
        device_id,
    )
    .await
}

async fn person_names(
    device_users: &DeviceUserRepository,
    device_id: Uuid,
) -> Result<HashMap<i64, (Uuid, Option<String>)>, EventError> {
    let rows = device_users
        .names_for_device(device_id)
        .await
        .map_err(map_db_error)?;
    Ok(rows
        .into_iter()
        .map(|(ref_user_id, user_id, username)| (ref_user_id, (user_id, username)))
        .collect())
}

fn person_from_names(
    names: &HashMap<i64, (Uuid, Option<String>)>,
    ref_user_id: Option<i64>,
) -> (Option<Uuid>, Option<String>) {
    let Some(ref_user_id) = ref_user_id else {
        return (None, None);
    };
    match names.get(&ref_user_id) {
        Some((user_id, username)) => (Some(*user_id), username.clone()),
        None => (None, None),
    }
}

fn page_needs_person_names(rows: &[crate::database::models::AccessEventRecord]) -> bool {
    let user_events = user_event_ids_with_field1_user();
    rows.iter().any(|row| {
        row.person_name.is_none()
            && row
                .event_id
                .as_deref()
                .is_some_and(|event_id| user_events.iter().any(|id| id == event_id))
    })
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
    match error {
        SecretError::Unavailable => EventError::SecretUnavailable,
        SecretError::Corrupt => EventError::SecretCorrupt,
    }
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
        advance_cursor, after_empty_batch, caught_up, fetch_step, map_secret_error, next_seq,
        occurred_label, person_from_names, EventCount, EventCursor, EventError, FetchStep,
        PollTracker, MAX_BATCHES_PER_FETCH,
    };
    use crate::common::{SecretError, SecretVault, WorkloadConfig};
    use crate::database::repositories::DeviceRepository;
    use crate::domains::devices::{create_device, set_device_password};
    use std::collections::HashMap;
    use std::collections::HashSet;
    use uuid::Uuid;

    #[test]
    fn corrupt_secret_is_not_reported_as_unavailable() {
        assert_eq!(
            map_secret_error(SecretError::Corrupt).to_string(),
            "SECRET_CORRUPT"
        );
        assert_eq!(
            map_secret_error(SecretError::Unavailable).to_string(),
            "SECRET_UNAVAILABLE"
        );
        assert_ne!(
            EventError::SecretCorrupt.to_string(),
            EventError::SecretUnavailable.to_string()
        );
    }

    #[test]
    fn one_visit_reads_at_most_one_hundred_events() {
        assert_eq!(MAX_BATCHES_PER_FETCH, 20);
        assert_eq!(crate::matrix::events::EVENT_BATCH_SIZE, 5);
        assert_eq!(
            MAX_BATCHES_PER_FETCH * u32::try_from(crate::matrix::events::EVENT_BATCH_SIZE).unwrap(),
            100
        );
    }

    #[tokio::test]
    #[ignore = "requires an isolated PostgreSQL database named in TEST_DATABASE_URL"]
    async fn password_reset_restores_decrypt_and_does_not_count_as_a_sync() {
        crate::database::run_isolated_test(|db| async move {
            let repo = DeviceRepository::new(db.pool());
            let original = SecretVault::from_key([4u8; 32]).expect("original");
            let replacement = SecretVault::from_key([5u8; 32]).expect("replacement");
            let created = create_device(
                &repo,
                &original,
                "Recovery Door",
                "203.0.113.20",
                Some(91),
                None,
                "admin",
                "alpha",
            )
            .await
            .expect("create");
            db.track_device(created.id);
            let stored = repo
                .find_by_id(created.id)
                .await
                .expect("load")
                .expect("row");
            assert_eq!(
                replacement.decrypt(&stored.password_ciphertext),
                Err(SecretError::Corrupt)
            );
            assert_eq!(
                map_secret_error(SecretError::Corrupt).to_string(),
                "SECRET_CORRUPT"
            );
            let tracker = PollTracker::new(WorkloadConfig::from_values(Some("4"), None));
            let now = chrono::Utc::now();
            tracker.select_due(
                now,
                &[super::schedule::ReaderRef {
                    id: created.id,
                    name: "Recovery Door".to_string(),
                }],
                &HashSet::new(),
            );
            tracker.record_failure(created.id, "SECRET_CORRUPT", now);
            let failed = tracker.snapshot();
            assert!(failed.devices[0].needs_operator);
            assert!(failed.devices[0].last_success.is_none());
            assert_eq!(failed.events_persisted, 0);
            assert_eq!(failed.last_cycle_reached, 0);
            set_device_password(&repo, &replacement, created.id, "beta")
                .await
                .expect("reset");
            let stored = repo
                .find_by_id(created.id)
                .await
                .expect("reload")
                .expect("row");
            assert_eq!(
                replacement
                    .decrypt(&stored.password_ciphertext)
                    .expect("decrypt"),
                "beta"
            );
            let after_reset = tracker.snapshot();
            assert!(after_reset.devices[0].last_success.is_none());
            assert_eq!(after_reset.events_persisted, 0);
            assert!(after_reset.last_checkpoint_at.is_none());
        })
        .await;
    }

    #[test]
    fn person_name_comes_from_the_preloaded_map() {
        let user_id = Uuid::nil();
        let mut names = HashMap::new();
        names.insert(42, (user_id, Some("Ada".to_string())));
        assert_eq!(
            person_from_names(&names, Some(42)),
            (Some(user_id), Some("Ada".to_string()))
        );
        assert_eq!(person_from_names(&names, Some(7)), (None, None));
        assert_eq!(person_from_names(&names, None), (None, None));
    }

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
