//! Which readers are due, how many may be contacted at once, and how long a
//! failing reader waits before the next attempt.
//!
//! One poll cycle runs at a time. A reader is in at most one fetch. Enrollment
//! stays outside this module: the caller passes the readers that are busy.

use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::common::{
    WorkloadConfig, FOREGROUND_POOL_RESERVE, MAX_POLL_CONCURRENCY, RECOMMENDED_CONCURRENCY,
};

const BACKOFF_BASE: Duration = Duration::from_secs(15);
const BACKOFF_CAP: Duration = Duration::from_secs(600);
const OPERATOR_AFTER_FAILURES: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReaderRef {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone)]
struct ReaderState {
    name: String,
    last_success: Option<DateTime<Utc>>,
    last_error: Option<String>,
    last_attempt: Option<DateTime<Utc>>,
    consecutive_failures: u32,
    next_retry_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
struct PollInner {
    config: WorkloadConfig,
    active_requests: usize,
    max_in_flight: usize,
    in_flight: HashSet<Uuid>,
    cycle_running: bool,
    configured_readers: usize,
    due_readers: usize,
    cooling_down: usize,
    events_persisted: u64,
    last_cycle_ms: Option<u64>,
    last_cycle_reached: usize,
    last_cycle_failed: usize,
    last_cycle_events: u32,
    visit_p50_ms: Option<u64>,
    visit_p95_ms: Option<u64>,
    queue_p50_ms: Option<u64>,
    decrypt_p50_ms: Option<u64>,
    http_p50_ms: Option<u64>,
    database_p50_ms: Option<u64>,
    failures: PollFailureCounts,
    last_checkpoint_reader: Option<String>,
    last_checkpoint_rollover: Option<i64>,
    last_checkpoint_seq: Option<i64>,
    last_checkpoint_at: Option<DateTime<Utc>>,
    pool_in_use: u32,
    pool_idle: u32,
    readers: HashMap<Uuid, ReaderState>,
}

/// In-memory poll book. A restart clears cooldowns and tries every reader again.
/// Event checkpoints stay in PostgreSQL, so a restart does not re-import events
/// that were already stored.
#[derive(Clone)]
pub struct PollTracker {
    inner: Arc<Mutex<PollInner>>,
}

impl PollTracker {
    pub fn new(config: WorkloadConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PollInner {
                config,
                active_requests: 0,
                max_in_flight: 0,
                in_flight: HashSet::new(),
                cycle_running: false,
                configured_readers: 0,
                due_readers: 0,
                cooling_down: 0,
                events_persisted: 0,
                last_cycle_ms: None,
                last_cycle_reached: 0,
                last_cycle_failed: 0,
                last_cycle_events: 0,
                visit_p50_ms: None,
                visit_p95_ms: None,
                queue_p50_ms: None,
                decrypt_p50_ms: None,
                http_p50_ms: None,
                database_p50_ms: None,
                failures: PollFailureCounts::default(),
                last_checkpoint_reader: None,
                last_checkpoint_rollover: None,
                last_checkpoint_seq: None,
                last_checkpoint_at: None,
                pool_in_use: 0,
                pool_idle: 0,
                readers: HashMap::new(),
            })),
        }
    }

    pub fn config(&self) -> WorkloadConfig {
        self.lock().config.clone()
    }

    pub fn try_begin_cycle(&self) -> bool {
        let mut inner = self.lock();
        if inner.cycle_running {
            return false;
        }
        inner.cycle_running = true;
        true
    }

    pub fn finish_cycle(&self, elapsed: Duration, configured: usize, due: usize, cooling: usize) {
        let mut inner = self.lock();
        inner.cycle_running = false;
        inner.configured_readers = configured;
        inner.due_readers = due;
        inner.cooling_down = cooling;
        inner.last_cycle_ms = Some(u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX));
    }

    /// Active readers that are not in cooldown, oldest attempt first.
    /// Duplicate ids are contacted once.
    pub fn select_due(
        &self,
        now: DateTime<Utc>,
        readers: &[ReaderRef],
        busy: &HashSet<Uuid>,
    ) -> Vec<Uuid> {
        let mut inner = self.lock();
        let mut due = Vec::new();
        let mut seen = HashSet::new();
        for reader in readers {
            if !seen.insert(reader.id) || busy.contains(&reader.id) {
                continue;
            }
            let state = inner
                .readers
                .entry(reader.id)
                .or_insert_with(|| ReaderState {
                    name: reader.name.clone(),
                    last_success: None,
                    last_error: None,
                    last_attempt: None,
                    consecutive_failures: 0,
                    next_retry_at: None,
                });
            if state.name != reader.name {
                state.name = reader.name.clone();
            }
            if state.next_retry_at.is_some_and(|retry| retry > now) {
                continue;
            }
            due.push(reader.id);
        }
        due.sort_by_key(|id| {
            inner
                .readers
                .get(id)
                .and_then(|state| state.last_attempt)
                .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        });
        due
    }

    /// Drop readers that are no longer active. An in-flight fetch is kept until it finishes.
    pub fn retain_listed(&self, listed: &HashSet<Uuid>) {
        let mut inner = self.lock();
        let in_flight = inner.in_flight.clone();
        inner
            .readers
            .retain(|id, _| listed.contains(id) || in_flight.contains(id));
    }

    #[cfg(test)]
    fn tracked_readers(&self) -> usize {
        self.lock().readers.len()
    }

    /// One in-flight fetch per reader. A second claim is refused until release.
    pub fn try_claim(&self, device_id: Uuid) -> bool {
        let mut inner = self.lock();
        if !inner.in_flight.insert(device_id) {
            return false;
        }
        inner.active_requests = inner.active_requests.saturating_add(1);
        inner.max_in_flight = inner.max_in_flight.max(inner.active_requests);
        true
    }

    pub fn release(&self, device_id: Uuid) {
        let mut inner = self.lock();
        if inner.in_flight.remove(&device_id) {
            inner.active_requests = inner.active_requests.saturating_sub(1);
        }
    }

    pub fn set_pool(&self, in_use: u32, idle: u32) {
        let mut inner = self.lock();
        inner.pool_in_use = in_use;
        inner.pool_idle = idle;
    }

    pub fn record_success(&self, device_id: Uuid, imported: u32, now: DateTime<Utc>) {
        let mut inner = self.lock();
        if let Some(state) = inner.readers.get_mut(&device_id) {
            state.last_success = Some(now);
            state.last_attempt = Some(now);
            state.last_error = None;
            state.consecutive_failures = 0;
            state.next_retry_at = None;
        }
        inner.events_persisted = inner.events_persisted.saturating_add(u64::from(imported));
    }

    /// Remember the cursor a visit confirmed. A failed visit does not call this.
    pub fn record_checkpoint(
        &self,
        device_id: Uuid,
        roll_over_count: i64,
        seq_number: i64,
        now: DateTime<Utc>,
    ) {
        let mut inner = self.lock();
        let name = inner
            .readers
            .get(&device_id)
            .map(|state| state.name.clone())
            .unwrap_or_else(|| device_id.to_string());
        inner.last_checkpoint_reader = Some(name);
        inner.last_checkpoint_rollover = Some(roll_over_count);
        inner.last_checkpoint_seq = Some(seq_number);
        inner.last_checkpoint_at = Some(now);
    }

    /// Aggregate one cycle. A reached visit retrieved data and persisted what it stored.
    /// A busy or inactive reader is neither a success nor a failure.
    pub fn record_visits(&self, visits: &[VisitSample]) {
        let mut inner = self.lock();
        let mut totals = Vec::new();
        let mut queues = Vec::new();
        let mut decrypts = Vec::new();
        let mut https = Vec::new();
        let mut databases = Vec::new();
        let mut reached = 0_usize;
        let mut failed = 0_usize;
        let mut events = 0_u32;
        let mut counts = PollFailureCounts::default();
        for visit in visits {
            if visit.code.as_deref() == Some("DEVICE_BUSY")
                || visit.code.as_deref() == Some("DEVICE_INACTIVE")
            {
                continue;
            }
            totals.push(visit.total_ms);
            queues.push(visit.queue_ms);
            if let Some(value) = visit.decrypt_ms {
                decrypts.push(value);
            }
            if let Some(value) = visit.http_ms {
                https.push(value);
            }
            if let Some(value) = visit.database_ms {
                databases.push(value);
            }
            if visit.reached {
                reached += 1;
                events = events.saturating_add(visit.imported);
            } else {
                failed += 1;
                match visit.code.as_deref() {
                    Some("SECRET_CORRUPT") => counts.secret_corrupt += 1,
                    Some("SECRET_UNAVAILABLE") => counts.secret_unavailable += 1,
                    Some("MATRIX_AUTH_FAILED") => counts.auth_failed += 1,
                    Some("MATRIX_TIMEOUT") => counts.timeout += 1,
                    Some("MATRIX_UNREACHABLE") => counts.unreachable += 1,
                    Some("DATABASE_UNAVAILABLE" | "DATABASE_ERROR") => counts.database += 1,
                    _ => counts.other += 1,
                }
            }
        }
        totals.sort_unstable();
        queues.sort_unstable();
        decrypts.sort_unstable();
        https.sort_unstable();
        databases.sort_unstable();
        inner.last_cycle_reached = reached;
        inner.last_cycle_failed = failed;
        inner.last_cycle_events = events;
        inner.visit_p50_ms = percentile(&totals, 50);
        inner.visit_p95_ms = percentile(&totals, 95);
        inner.queue_p50_ms = percentile(&queues, 50);
        inner.decrypt_p50_ms = percentile(&decrypts, 50);
        inner.http_p50_ms = percentile(&https, 50);
        inner.database_p50_ms = percentile(&databases, 50);
        inner.failures = counts;
    }

    /// Retryable reader failures wait with exponential backoff and jitter.
    /// A busy or inactive reader is not a failure.
    pub fn record_failure(&self, device_id: Uuid, code: &str, now: DateTime<Utc>) {
        let mut inner = self.lock();
        let Some(state) = inner.readers.get_mut(&device_id) else {
            return;
        };
        state.last_attempt = Some(now);
        state.last_error = Some(code.to_string());
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        let delay = retry_delay(
            state.consecutive_failures,
            jitter_ms(device_id, state.consecutive_failures),
        );
        state.next_retry_at = Some(now + chrono::Duration::from_std(delay).unwrap_or_default());
    }

    pub fn snapshot(&self) -> PollStatus {
        let inner = self.lock();
        let mut devices: Vec<DevicePollStatus> = inner
            .readers
            .iter()
            .filter(|(_, state)| state.last_error.is_some() || state.next_retry_at.is_some())
            .map(|(id, state)| DevicePollStatus {
                device_id: *id,
                device_name: state.name.clone(),
                last_success: state.last_success,
                last_error: state.last_error.clone(),
                next_retry_at: state.next_retry_at,
                consecutive_failures: state.consecutive_failures,
                needs_operator: asks_for_operator(state),
            })
            .collect();
        devices.sort_by(|left, right| {
            right
                .consecutive_failures
                .cmp(&left.consecutive_failures)
                .then_with(|| left.device_name.cmp(&right.device_name))
        });
        devices.truncate(40);
        let failing = inner
            .readers
            .values()
            .filter(|state| state.consecutive_failures > 0)
            .count();
        PollStatus {
            concurrency: inner.config.concurrency,
            recommended_concurrency: RECOMMENDED_CONCURRENCY,
            max_concurrency: MAX_POLL_CONCURRENCY,
            interval_secs: inner.config.interval.as_secs(),
            pool_size: inner.config.pool_size,
            foreground_reserve: FOREGROUND_POOL_RESERVE,
            guidance: inner.config.guidance(),
            active_requests: inner.active_requests,
            configured_readers: inner.configured_readers,
            due_readers: inner.due_readers,
            cooling_down: inner.cooling_down,
            failing,
            events_persisted: inner.events_persisted,
            last_cycle_ms: inner.last_cycle_ms,
            last_cycle_reached: inner.last_cycle_reached,
            last_cycle_failed: inner.last_cycle_failed,
            last_cycle_events: inner.last_cycle_events,
            visit_p50_ms: inner.visit_p50_ms,
            visit_p95_ms: inner.visit_p95_ms,
            queue_p50_ms: inner.queue_p50_ms,
            decrypt_p50_ms: inner.decrypt_p50_ms,
            http_p50_ms: inner.http_p50_ms,
            database_p50_ms: inner.database_p50_ms,
            max_in_flight: inner.max_in_flight,
            failures: inner.failures,
            last_checkpoint_reader: inner.last_checkpoint_reader.clone(),
            last_checkpoint_rollover: inner.last_checkpoint_rollover,
            last_checkpoint_seq: inner.last_checkpoint_seq,
            last_checkpoint_at: inner.last_checkpoint_at,
            pool_in_use: inner.pool_in_use,
            pool_idle: inner.pool_idle,
            devices,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PollInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollStatus {
    pub concurrency: usize,
    pub recommended_concurrency: usize,
    pub max_concurrency: usize,
    pub interval_secs: u64,
    pub pool_size: u32,
    pub foreground_reserve: u32,
    pub guidance: String,
    pub active_requests: usize,
    pub configured_readers: usize,
    pub due_readers: usize,
    pub cooling_down: usize,
    pub failing: usize,
    pub events_persisted: u64,
    pub last_cycle_ms: Option<u64>,
    /// Readers whose visit retrieved device data and finished its writes.
    pub last_cycle_reached: usize,
    /// Readers whose visit failed. A busy or inactive reader is not included.
    pub last_cycle_failed: usize,
    pub last_cycle_events: u32,
    pub visit_p50_ms: Option<u64>,
    pub visit_p95_ms: Option<u64>,
    pub queue_p50_ms: Option<u64>,
    pub decrypt_p50_ms: Option<u64>,
    pub http_p50_ms: Option<u64>,
    pub database_p50_ms: Option<u64>,
    pub max_in_flight: usize,
    pub failures: PollFailureCounts,
    pub last_checkpoint_reader: Option<String>,
    pub last_checkpoint_rollover: Option<i64>,
    pub last_checkpoint_seq: Option<i64>,
    pub last_checkpoint_at: Option<DateTime<Utc>>,
    pub pool_in_use: u32,
    pub pool_idle: u32,
    pub devices: Vec<DevicePollStatus>,
}

/// Safe failure categories for the last cycle. Counts only, no secrets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollFailureCounts {
    pub secret_corrupt: u32,
    pub secret_unavailable: u32,
    pub auth_failed: u32,
    pub timeout: u32,
    pub unreachable: u32,
    pub database: u32,
    pub other: u32,
}

/// One reader visit, used to build the cycle aggregate.
#[derive(Debug, Clone)]
pub struct VisitSample {
    pub reached: bool,
    pub code: Option<String>,
    pub imported: u32,
    pub queue_ms: u64,
    pub total_ms: u64,
    pub decrypt_ms: Option<u64>,
    pub http_ms: Option<u64>,
    pub database_ms: Option<u64>,
}

pub struct PolledReader<T> {
    pub id: Uuid,
    pub output: T,
    pub queue_ms: u64,
    pub visit_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePollStatus {
    pub device_id: Uuid,
    pub device_name: String,
    pub last_success: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub needs_operator: bool,
}

pub fn retry_delay(failures: u32, jitter_ms: u64) -> Duration {
    let shift = failures.saturating_sub(1).min(6);
    let base_ms = BACKOFF_BASE.as_millis() as u64;
    let grown = base_ms.saturating_mul(1u64 << shift);
    let capped = grown.min(BACKOFF_CAP.as_millis() as u64);
    let spread = (capped / 5).max(1);
    Duration::from_millis(capped.saturating_add(jitter_ms % spread))
}

fn jitter_ms(device_id: Uuid, failures: u32) -> u64 {
    let mut mixed = u64::from(failures);
    for byte in device_id.as_bytes() {
        mixed = mixed.wrapping_mul(31).wrapping_add(u64::from(*byte));
    }
    mixed
}

fn asks_for_operator(state: &ReaderState) -> bool {
    match state.last_error.as_deref() {
        Some("SECRET_CORRUPT" | "SECRET_UNAVAILABLE") => true,
        Some("MATRIX_AUTH_FAILED") => state.consecutive_failures >= OPERATOR_AFTER_FAILURES,
        _ => false,
    }
}

fn percentile(samples: &[u64], percent: u8) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let index = (samples.len() - 1) * usize::from(percent) / 100;
    Some(samples[index])
}

fn elapsed_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Contact `ids` with at most `concurrency` fetches in flight.
/// One id is passed once. Workers are the concurrency limit: waiting readers
/// stay in a queue instead of each holding a task. A failure inside `fetch`
/// does not cancel the others.
pub async fn poll_wave<F, Fut, T>(
    ids: Vec<Uuid>,
    concurrency: usize,
    fetch: F,
) -> (Vec<PolledReader<Result<T, String>>>, usize)
where
    F: Fn(Uuid) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, String>> + Send + 'static,
    T: Send + 'static,
{
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for id in ids {
        if seen.insert(id) {
            unique.push(id);
        }
    }
    if unique.is_empty() {
        return (Vec::new(), 0);
    }
    let concurrency = concurrency.clamp(1, MAX_POLL_CONCURRENCY).min(unique.len());
    let queue = Arc::new(std::sync::Mutex::new(VecDeque::from(unique.clone())));
    let fetch = Arc::new(fetch);
    let wave_started = Instant::now();
    let mut tasks = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let queue = Arc::clone(&queue);
        let fetch = Arc::clone(&fetch);
        tasks.push(tokio::spawn(async move {
            let mut done = Vec::new();
            loop {
                let next = {
                    let mut guard = queue
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    guard.pop_front()
                };
                let Some(id) = next else {
                    break;
                };
                let popped = Instant::now();
                let queue_ms = elapsed_ms(popped.saturating_duration_since(wave_started));
                let output = fetch(id).await;
                let visit_ms = elapsed_ms(popped.elapsed());
                done.push(PolledReader {
                    id,
                    output,
                    queue_ms,
                    visit_ms,
                });
            }
            done
        }));
    }
    let mut finished = Vec::with_capacity(unique.len());
    let mut returned = HashSet::new();
    for task in tasks {
        if let Ok(batch) = task.await {
            for item in batch {
                returned.insert(item.id);
                finished.push(item);
            }
        }
    }
    for id in unique {
        if returned.insert(id) {
            finished.push(PolledReader {
                id,
                output: Err("POLL_TASK_FAILED".to_string()),
                queue_ms: 0,
                visit_ms: 0,
            });
        }
    }
    (finished, concurrency)
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct WaveStats {
    pub completed: usize,
    pub failed: usize,
    pub max_in_flight: usize,
    pub workers: usize,
    pub elapsed: Duration,
}

/// Simulator for the real wave scheduler. Delays stand in for device HTTP.
/// These numbers are not physical-reader capacity.
#[cfg(test)]
async fn simulate_wave(
    readers: usize,
    concurrency: usize,
    slow: usize,
    slow_delay: Duration,
    fast_delay: Duration,
) -> WaveStats {
    let ids: Vec<Uuid> = (0..readers).map(|_| Uuid::new_v4()).collect();
    let slow_ids: HashSet<Uuid> = ids.iter().take(slow).copied().collect();
    let in_flight = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let peak = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let started = Instant::now();
    let in_flight_fetch = Arc::clone(&in_flight);
    let peak_fetch = Arc::clone(&peak);
    let slow_ids_fetch = slow_ids.clone();
    let (finished, workers) = poll_wave(ids, concurrency, move |id| {
        let in_flight = Arc::clone(&in_flight_fetch);
        let peak = Arc::clone(&peak_fetch);
        let is_slow = slow_ids_fetch.contains(&id);
        async move {
            let now = in_flight.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            peak.fetch_max(now, std::sync::atomic::Ordering::SeqCst);
            let delay = if is_slow { slow_delay } else { fast_delay };
            tokio::time::sleep(delay).await;
            in_flight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            if is_slow {
                Err("MATRIX_TIMEOUT".to_string())
            } else {
                Ok(1)
            }
        }
    })
    .await;
    WaveStats {
        completed: finished.iter().filter(|item| item.output.is_ok()).count(),
        failed: finished.iter().filter(|item| item.output.is_err()).count(),
        max_in_flight: peak.load(std::sync::atomic::Ordering::SeqCst),
        workers,
        elapsed: started.elapsed(),
    }
}

#[cfg(test)]
fn resident_bytes() -> u64 {
    #[repr(C)]
    struct TaskBasic {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        user_seconds: i32,
        user_micros: i32,
        system_seconds: i32,
        system_micros: i32,
        policy: i32,
        suspend_count: i32,
    }
    unsafe extern "C" {
        fn mach_task_self() -> u32;
        fn task_info(target: u32, flavor: u32, info: *mut i32, count: *mut u32) -> i32;
    }
    const MACH_TASK_BASIC_INFO: u32 = 20;
    unsafe {
        let mut info = std::mem::zeroed::<TaskBasic>();
        let mut count = (std::mem::size_of::<TaskBasic>() / std::mem::size_of::<u32>()) as u32;
        let rc = task_info(
            mach_task_self(),
            MACH_TASK_BASIC_INFO,
            &mut info as *mut TaskBasic as *mut i32,
            &mut count,
        );
        if rc == 0 {
            info.resident_size
        } else {
            0
        }
    }
}

#[cfg(test)]
fn user_cpu_ms() -> u64 {
    #[repr(C)]
    struct Timeval {
        sec: i64,
        usec: i32,
        pad: i32,
    }
    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut u8) -> i32;
    }
    unsafe {
        let mut usage = [0u8; 256];
        if getrusage(0, usage.as_mut_ptr()) != 0 {
            return 0;
        }
        let timeval = &*(usage.as_ptr() as *const Timeval);
        (timeval.sec as u64).saturating_mul(1000) + (timeval.usec as u64) / 1000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readers(count: usize) -> Vec<ReaderRef> {
        (0..count)
            .map(|index| ReaderRef {
                id: Uuid::from_u128(index as u128 + 1),
                name: format!("Door {index}"),
            })
            .collect()
    }

    #[test]
    fn cooldown_skips_a_failing_reader_and_keeps_the_others() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(Some("4"), None));
        let listed = readers(3);
        let now = Utc::now();
        let due = tracker.select_due(now, &listed, &HashSet::new());
        assert_eq!(due.len(), 3);
        tracker.record_failure(listed[0].id, "MATRIX_TIMEOUT", now);
        let again = tracker.select_due(now, &listed, &HashSet::new());
        assert!(!again.contains(&listed[0].id));
        assert_eq!(again.len(), 2);
    }

    #[tokio::test]
    async fn enrollment_on_one_reader_does_not_add_workers_or_block_the_others() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(Some("4"), None));
        let listed = readers(8);
        let mut busy = HashSet::new();
        busy.insert(listed[0].id);
        let due = tracker.select_due(Utc::now(), &listed, &busy);
        assert!(!due.contains(&listed[0].id));
        assert_eq!(due.len(), 7);
        let started = Instant::now();
        let (finished, workers) = poll_wave(due, 4, |_id| async move {
            tokio::time::sleep(Duration::from_millis(15)).await;
            Ok::<u32, String>(0)
        })
        .await;
        let elapsed = started.elapsed();
        eprintln!(
            "enrollment_overlap readers=7 concurrency=4 workers={workers} elapsed_ms={}",
            elapsed.as_millis()
        );
        assert_eq!(workers, 4);
        assert_eq!(finished.len(), 7);
        assert!(finished.iter().all(|item| item.output.is_ok()));
        assert!(elapsed < Duration::from_millis(200));
        assert!(tracker.snapshot().active_requests <= 4);
    }

    #[test]
    fn enrollment_and_duplicates_are_not_polled() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(Some("4"), None));
        let listed = vec![
            readers(1)[0].clone(),
            readers(2)[1].clone(),
            readers(2)[1].clone(),
        ];
        let mut busy = HashSet::new();
        busy.insert(listed[0].id);
        let due = tracker.select_due(Utc::now(), &listed, &busy);
        assert_eq!(due, vec![listed[1].id]);
    }

    #[test]
    fn a_new_tracker_forgets_cooldown_after_restart() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        let listed = readers(1);
        let now = Utc::now();
        tracker.select_due(now, &listed, &HashSet::new());
        tracker.record_failure(listed[0].id, "MATRIX_UNREACHABLE", now);
        let restarted = PollTracker::new(WorkloadConfig::from_values(None, None));
        let due = restarted.select_due(now, &listed, &HashSet::new());
        assert_eq!(due, vec![listed[0].id]);
    }

    #[test]
    fn repeated_auth_failure_asks_for_an_operator() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        let listed = readers(1);
        let now = Utc::now();
        tracker.select_due(now, &listed, &HashSet::new());
        for _ in 0..3 {
            tracker.record_failure(listed[0].id, "MATRIX_AUTH_FAILED", now);
        }
        let status = tracker.snapshot();
        assert!(status.devices[0].needs_operator);
        assert_eq!(
            status.devices[0].last_error.as_deref(),
            Some("MATRIX_AUTH_FAILED")
        );
    }

    #[test]
    fn a_corrupt_secret_asks_for_the_password_and_is_not_a_success() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        let listed = readers(1);
        let now = Utc::now();
        tracker.select_due(now, &listed, &HashSet::new());
        tracker.record_failure(listed[0].id, "SECRET_CORRUPT", now);
        let status = tracker.snapshot();
        assert!(status.devices[0].needs_operator);
        assert_eq!(
            status.devices[0].last_error.as_deref(),
            Some("SECRET_CORRUPT")
        );
        assert!(status.devices[0].last_success.is_none());
        assert_eq!(status.events_persisted, 0);
        assert!(status.last_checkpoint_at.is_none());
    }

    #[test]
    fn a_reached_visit_is_not_the_same_as_a_finished_attempt() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        let listed = readers(1);
        let now = Utc::now();
        tracker.select_due(now, &listed, &HashSet::new());
        tracker.record_visits(&[
            VisitSample {
                reached: true,
                code: None,
                imported: 3,
                queue_ms: 4,
                total_ms: 40,
                decrypt_ms: Some(1),
                http_ms: Some(30),
                database_ms: Some(5),
            },
            VisitSample {
                reached: false,
                code: Some("SECRET_CORRUPT".to_string()),
                imported: 0,
                queue_ms: 0,
                total_ms: 1,
                decrypt_ms: None,
                http_ms: None,
                database_ms: None,
            },
            VisitSample {
                reached: false,
                code: Some("DEVICE_BUSY".to_string()),
                imported: 0,
                queue_ms: 0,
                total_ms: 0,
                decrypt_ms: None,
                http_ms: None,
                database_ms: None,
            },
        ]);
        tracker.record_success(listed[0].id, 3, now);
        tracker.record_checkpoint(listed[0].id, 2, 15, now);
        let status = tracker.snapshot();
        assert_eq!(status.last_cycle_reached, 1);
        assert_eq!(status.last_cycle_failed, 1);
        assert_eq!(status.last_cycle_events, 3);
        assert_eq!(status.failures.secret_corrupt, 1);
        assert_eq!(status.events_persisted, 3);
        assert_eq!(status.last_checkpoint_rollover, Some(2));
        assert_eq!(status.last_checkpoint_seq, Some(15));
        assert_eq!(status.last_checkpoint_reader.as_deref(), Some("Door 0"));
        assert_eq!(status.visit_p50_ms, Some(1));
        assert_eq!(status.http_p50_ms, Some(30));
    }

    #[test]
    fn one_reader_cannot_be_claimed_twice() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        let id = Uuid::from_u128(7);
        assert!(tracker.try_claim(id));
        assert!(!tracker.try_claim(id));
        assert_eq!(tracker.snapshot().active_requests, 1);
        tracker.release(id);
        assert!(tracker.try_claim(id));
    }

    #[test]
    fn oldest_attempt_is_selected_first() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        let listed = readers(2);
        let now = Utc::now();
        tracker.select_due(now, &listed, &HashSet::new());
        tracker.record_success(listed[1].id, 0, now);
        tracker.record_success(listed[0].id, 0, now + chrono::Duration::seconds(5));
        let due = tracker.select_due(
            now + chrono::Duration::seconds(10),
            &listed,
            &HashSet::new(),
        );
        assert_eq!(due[0], listed[1].id);
    }

    #[test]
    fn overlapping_cycles_are_refused() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        assert!(tracker.try_begin_cycle());
        assert!(!tracker.try_begin_cycle());
        tracker.finish_cycle(Duration::from_millis(10), 1, 1, 0);
        assert!(tracker.try_begin_cycle());
    }

    #[tokio::test]
    async fn sustained_waves_keep_a_bounded_task_count() {
        let before = resident_bytes();
        let cpu_before = user_cpu_ms();
        let mut spawned = 0usize;
        for concurrency in [2usize, 4, 8, 16, 40] {
            for _ in 0..20 {
                let stats = simulate_wave(
                    40,
                    concurrency,
                    0,
                    Duration::from_millis(1),
                    Duration::from_millis(1),
                )
                .await;
                assert_eq!(stats.completed, 40);
                assert_eq!(stats.workers, concurrency);
                assert!(stats.max_in_flight <= concurrency);
                spawned += stats.workers;
            }
        }
        let after = resident_bytes();
        let cpu_ms = user_cpu_ms().saturating_sub(cpu_before);
        eprintln!(
            "sustained_waves cycles=100 readers=40 spawned_tasks={spawned} resident_before={before} resident_after={after} user_cpu_ms={cpu_ms}"
        );
        assert_eq!(spawned, 20 * (2 + 4 + 8 + 16 + 40));
        assert!(after > 0, "mach resident size was not available");
        assert!(
            after < before.saturating_add(16 * 1024 * 1024),
            "resident grew by more than 16 MiB: before {before} after {after}"
        );
    }

    #[test]
    fn removed_readers_leave_the_poll_book() {
        let tracker = PollTracker::new(WorkloadConfig::from_values(None, None));
        let now = Utc::now();
        for wave in 0..30 {
            let listed = (0..40)
                .map(|index| ReaderRef {
                    id: Uuid::from_u128((wave * 40 + index + 1) as u128),
                    name: format!("Door {index}"),
                })
                .collect::<Vec<_>>();
            tracker.select_due(now, &listed, &HashSet::new());
            let ids = listed.iter().map(|reader| reader.id).collect();
            tracker.retain_listed(&ids);
            assert_eq!(tracker.tracked_readers(), 40);
        }
    }

    fn percentile(samples: &[u128], percent: u8) -> u128 {
        if samples.is_empty() {
            return 0;
        }
        let index = (samples.len() - 1) * usize::from(percent) / 100;
        samples[index]
    }

    fn median(samples: &mut [u128]) -> u128 {
        samples.sort_unstable();
        percentile(samples, 50)
    }

    #[tokio::test]
    async fn concurrency_trials_record_cycle_time() {
        for (label, slow, slow_delay, fast_delay) in [
            (
                "healthy",
                0usize,
                Duration::from_millis(1),
                Duration::from_millis(30),
            ),
            (
                "mixed",
                8usize,
                Duration::from_millis(80),
                Duration::from_millis(5),
            ),
        ] {
            for concurrency in [2usize, 4, 8, 16, 40] {
                let mut cycles = Vec::new();
                let mut peaks = Vec::new();
                for _ in 0..5 {
                    let stats = simulate_wave(40, concurrency, slow, slow_delay, fast_delay).await;
                    let expected_ok = 40 - slow;
                    assert_eq!(stats.completed, expected_ok, "{label} {concurrency}");
                    assert_eq!(stats.failed, slow, "{label} {concurrency}");
                    assert!(stats.max_in_flight <= concurrency, "{label} {concurrency}");
                    assert_eq!(stats.workers, concurrency, "{label} {concurrency}");
                    cycles.push(stats.elapsed.as_millis());
                    peaks.push(stats.max_in_flight as u128);
                }
                let low = *cycles.iter().min().unwrap_or(&0);
                let high = *cycles.iter().max().unwrap_or(&0);
                let mid = median(&mut cycles);
                eprintln!(
                    "concurrency_trial label={label} concurrency={concurrency} trials=5 cycle_ms_median={mid} cycle_ms_min={low} cycle_ms_max={high} peak_max={}",
                    peaks.iter().copied().max().unwrap_or(0)
                );
            }
        }
    }

    #[tokio::test]
    async fn concurrency_limits_hold_for_forty_simulated_readers() {
        for concurrency in [2usize, 4, 8, 16, 40] {
            let stats = simulate_wave(
                40,
                concurrency,
                0,
                Duration::from_millis(1),
                Duration::from_millis(30),
            )
            .await;
            eprintln!(
                "simulated healthy readers=40 concurrency={concurrency} completed={} peak={} elapsed_ms={}",
                stats.completed,
                stats.max_in_flight,
                stats.elapsed.as_millis()
            );
            assert_eq!(stats.completed, 40, "concurrency {concurrency}");
            assert_eq!(stats.failed, 0, "concurrency {concurrency}");
            assert!(
                stats.max_in_flight <= concurrency,
                "peak {} exceeded {concurrency}",
                stats.max_in_flight
            );
            assert!(
                stats.max_in_flight >= concurrency.min(40) / 2,
                "peak {} was too low for concurrency {concurrency}",
                stats.max_in_flight
            );
        }
    }

    #[tokio::test]
    async fn slow_readers_do_not_stop_the_rest_of_the_wave() {
        let stats = simulate_wave(
            40,
            8,
            8,
            Duration::from_millis(80),
            Duration::from_millis(5),
        )
        .await;
        eprintln!(
            "simulated mixed readers=40 concurrency=8 slow=8 completed={} failed={} peak={} elapsed_ms={}",
            stats.completed,
            stats.failed,
            stats.max_in_flight,
            stats.elapsed.as_millis()
        );
        assert_eq!(stats.completed, 32);
        assert_eq!(stats.failed, 8);
        assert!(stats.max_in_flight <= 8);
        assert!(stats.elapsed < Duration::from_millis(500));
    }

    #[tokio::test]
    async fn one_failure_does_not_drop_the_other_readers() {
        let ids = vec![Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)];
        let (finished, workers) = poll_wave(ids.clone(), 3, |id| async move {
            if id == Uuid::from_u128(2) {
                Err("MATRIX_UNREACHABLE".to_string())
            } else {
                Ok(2)
            }
        })
        .await;
        assert_eq!(workers, 3);
        assert_eq!(finished.len(), 3);
        assert_eq!(
            finished.iter().filter(|item| item.output.is_ok()).count(),
            2
        );
    }
}
