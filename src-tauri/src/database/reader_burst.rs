//! Dummy in-and-out burst. No Matrix passwords and no device HTTP.
//!
//! Thirty readers each store 100 entries and 100 exits at the same time,
//! in the same five-event batches the poll uses. A second pass stores a
//! full day of 20,000 events. Ignored by a normal `cargo test`.

use std::time::Instant;

use chrono::Utc;
use uuid::Uuid;

use super::models::AccessEventWrite;
use super::repositories::AccessEventRepository;
use super::{connect_and_migrate_with_limit, writable_test_config};

const READERS: usize = 30;
const USERS_PER_READER: i64 = 100;
const DAY_EVENTS: i64 = 20_000;
/// Same size as one documented `getevent` batch.
const BATCH: usize = 5;

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing at a database whose name ends with _test"]
async fn thirty_readers_store_in_and_out_bursts() {
    let config = writable_test_config().expect("TEST_DATABASE_URL");
    let pool = connect_and_migrate_with_limit(&config, 36)
        .await
        .expect("test database");
    sqlx::query("DELETE FROM devices WHERE device_name LIKE 'Load Reader %'")
        .execute(&pool)
        .await
        .expect("clear previous load readers");

    let now = Utc::now();
    let mut readers = Vec::with_capacity(READERS);
    for index in 0..READERS {
        let id = Uuid::new_v4();
        let name = format!("Load Reader {}", index + 1);
        let host = format!("192.0.2.{}", index + 1);
        sqlx::query(
            r#"
            INSERT INTO devices (
                id, device_name, host, port, username, password_ciphertext,
                status, connection_status, created_at, updated_at
            )
            VALUES ($1, $2, $3, 80, 'load', $4, 'active', 'unknown', $5, $5)
            "#,
        )
        .bind(id)
        .bind(&name)
        .bind(&host)
        .bind(vec![0_u8; 32])
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert load reader");
        readers.push((id, name));
    }

    let burst_started = Instant::now();
    let mut burst_tasks = Vec::with_capacity(READERS);
    for (device_id, device_name) in readers.clone() {
        let pool = pool.clone();
        burst_tasks.push(tokio::spawn(async move {
            store_direction_pairs(&pool, device_id, &device_name, 1, USERS_PER_READER).await
        }));
    }
    let mut burst_events = 0_u64;
    for task in burst_tasks {
        burst_events += task.await.expect("burst task");
    }
    let burst_ms = burst_started.elapsed().as_millis();

    let day_started = Instant::now();
    let per_reader = DAY_EVENTS / READERS as i64;
    let remainder = DAY_EVENTS % READERS as i64;
    let mut day_tasks = Vec::with_capacity(READERS);
    for (index, (device_id, device_name)) in readers.iter().cloned().enumerate() {
        let extra = if (index as i64) < remainder { 1 } else { 0 };
        let count = per_reader + extra;
        let pool = pool.clone();
        day_tasks.push(tokio::spawn(async move {
            store_events(&pool, device_id, &device_name, 10_000, count).await
        }));
    }
    let mut day_events = 0_u64;
    for task in day_tasks {
        day_events += task.await.expect("day task");
    }
    let day_ms = day_started.elapsed().as_millis();

    let list_started = Instant::now();
    let listed = AccessEventRepository::new(pool.clone())
        .list(None, 200)
        .await
        .expect("list recent events");
    let list_ms = list_started.elapsed().as_millis();

    let stored: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM events e JOIN devices d ON d.id = e.device_id WHERE d.device_name LIKE 'Load Reader %'",
    )
    .fetch_one(&pool)
    .await
    .expect("count events");
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&pool)
        .await
        .expect("postgres version");
    let shared_buffers: String = sqlx::query_scalar("SHOW shared_buffers")
        .fetch_one(&pool)
        .await
        .expect("shared_buffers");

    sqlx::query("DELETE FROM devices WHERE device_name LIKE 'Load Reader %'")
        .execute(&pool)
        .await
        .expect("remove load readers");

    let expected_burst = (READERS as u64) * (USERS_PER_READER as u64) * 2;
    assert_eq!(burst_events, expected_burst);
    assert_eq!(day_events, DAY_EVENTS as u64);
    assert_eq!(stored, expected_burst as i64 + DAY_EVENTS);
    assert_eq!(listed.len(), 200);

    eprintln!(
        "burst readers={READERS} events={burst_events} ms={burst_ms} per_sec={}",
        events_per_second(burst_events, burst_ms)
    );
    eprintln!(
        "day readers={READERS} events={day_events} ms={day_ms} per_sec={}",
        events_per_second(day_events, day_ms)
    );
    eprintln!("recent_list rows=200 ms={list_ms}");
    eprintln!("postgres version={version}");
    eprintln!("postgres shared_buffers={shared_buffers}");
}

async fn store_direction_pairs(
    pool: &super::DbPool,
    device_id: Uuid,
    device_name: &str,
    seq_start: i64,
    users: i64,
) -> u64 {
    let mut records = Vec::with_capacity((users as usize) * 2);
    let mut seq = seq_start;
    for user in 1..=users {
        records.push(event(device_id, device_name, seq, user, "in"));
        seq += 1;
        records.push(event(device_id, device_name, seq, user, "out"));
        seq += 1;
    }
    persist_chunks(pool, device_id, &records).await
}

async fn store_events(
    pool: &super::DbPool,
    device_id: Uuid,
    device_name: &str,
    seq_start: i64,
    count: i64,
) -> u64 {
    let mut records = Vec::with_capacity(count as usize);
    for offset in 0..count {
        let user = (offset % USERS_PER_READER) + 1;
        let direction = if offset % 2 == 0 { "in" } else { "out" };
        records.push(event(
            device_id,
            device_name,
            seq_start + offset,
            user,
            direction,
        ));
    }
    persist_chunks(pool, device_id, &records).await
}

async fn persist_chunks(
    pool: &super::DbPool,
    device_id: Uuid,
    records: &[AccessEventWrite],
) -> u64 {
    let events = AccessEventRepository::new(pool.clone());
    let mut inserted = 0_u64;
    for chunk in records.chunks(BATCH) {
        let seq = chunk.last().map(|row| row.seq_number).unwrap_or(0);
        inserted += events
            .persist_batch_and_cursor(chunk, device_id, 1, seq, Utc::now())
            .await
            .expect("persist batch");
    }
    inserted
}

fn event(
    device_id: Uuid,
    device_name: &str,
    seq_number: i64,
    user: i64,
    direction: &str,
) -> AccessEventWrite {
    AccessEventWrite {
        id: Uuid::new_v4(),
        device_id,
        device_name: device_name.to_string(),
        roll_over_count: 1,
        seq_number,
        event_id: Some("101".to_string()),
        device_date: "21102026".to_string(),
        device_time: "090000".to_string(),
        user_id: None,
        person_name: Some(format!("User {user}")),
        ref_user_id: Some(user),
        detail_1: Some(user.to_string()),
        detail_2: Some(direction.to_string()),
        detail_3: None,
        detail_4: None,
        detail_5: None,
        ingested_at: Utc::now(),
    }
}

fn events_per_second(events: u64, elapsed_ms: u128) -> u64 {
    if elapsed_ms == 0 {
        return events;
    }
    events * 1000 / u64::try_from(elapsed_ms).unwrap_or(u64::MAX)
}
