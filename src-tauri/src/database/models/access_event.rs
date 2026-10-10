use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct AccessEventRecord {
    pub id: Uuid,
    pub device_id: Uuid,
    pub device_name: String,
    pub roll_over_count: i32,
    pub seq_number: i64,
    pub event_id: Option<String>,
    pub device_date: String,
    pub device_time: String,
    pub user_id: Option<Uuid>,
    pub person_name: Option<String>,
    pub ref_user_id: Option<i64>,
    pub detail_1: Option<String>,
    pub detail_2: Option<String>,
    pub detail_3: Option<String>,
    pub detail_4: Option<String>,
    pub detail_5: Option<String>,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AccessEventWrite {
    pub id: Uuid,
    pub device_id: Uuid,
    pub device_name: String,
    pub roll_over_count: i32,
    pub seq_number: i64,
    pub event_id: Option<String>,
    pub device_date: String,
    pub device_time: String,
    pub user_id: Option<Uuid>,
    pub person_name: Option<String>,
    pub ref_user_id: Option<i64>,
    pub detail_1: Option<String>,
    pub detail_2: Option<String>,
    pub detail_3: Option<String>,
    pub detail_4: Option<String>,
    pub detail_5: Option<String>,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, FromRow)]
pub struct DeviceEventCursorRecord {
    pub device_id: Uuid,
    pub roll_over_count: i32,
    pub seq_number: i64,
}
