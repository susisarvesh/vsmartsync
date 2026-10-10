//! Excel import of people. The sheet is accepted only when every row is valid.
//! Nothing is written to a device.

use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use calamine::{open_workbook_auto_from_rs, Data, Reader};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::models::UserRecord;
use crate::database::repositories::UserRepository;
use crate::database::DatabaseError;

use super::{normalize_username, UserError, UserStatus};

const MAX_IMPORT_BYTES: usize = 2 * 1024 * 1024;
const MAX_SHORT_NAME: usize = 15;
const MAX_FULL_NAME: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUsersResult {
    pub imported: u32,
    pub errors: Vec<ImportRowError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRowError {
    pub row: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportedUser {
    row: u32,
    matrix_user_id: String,
    username: String,
    short_name: String,
    full_name: Option<String>,
    reference_id: i64,
    active: bool,
}

enum Column {
    Id,
    Name,
    ShortName,
    ReferenceId,
    FullName,
    Active,
}

pub async fn import_users(
    repo: &UserRepository,
    bytes: &[u8],
) -> Result<ImportUsersResult, UserError> {
    if bytes.is_empty() || bytes.len() > MAX_IMPORT_BYTES {
        return Err(UserError::ImportUnreadable);
    }
    let rows = read_sheet(bytes).map_err(|_| UserError::ImportUnreadable)?;
    let parsed = parse_user_rows(&rows);
    if !parsed.errors.is_empty() {
        return Ok(ImportUsersResult {
            imported: 0,
            errors: parsed.errors,
        });
    }
    if parsed.users.is_empty() {
        return Ok(ImportUsersResult {
            imported: 0,
            errors: vec![ImportRowError {
                row: 1,
                message: "The sheet has no user rows.".into(),
            }],
        });
    }

    let ids: Vec<String> = parsed
        .users
        .iter()
        .map(|user| user.matrix_user_id.clone())
        .collect();
    let refs: Vec<i64> = parsed.users.iter().map(|user| user.reference_id).collect();
    let occupied = repo
        .occupied_import_keys(&ids, &refs)
        .await
        .map_err(map_db_error)?;
    let taken_ids: HashSet<String> = occupied.iter().filter_map(|(id, _)| id.clone()).collect();
    let taken_refs: HashSet<i64> = occupied
        .iter()
        .filter_map(|(_, reference)| *reference)
        .collect();
    let mut errors = Vec::new();
    for user in &parsed.users {
        if taken_ids.contains(&user.matrix_user_id) {
            errors.push(ImportRowError {
                row: user.row,
                message: format!("ID {} is already stored.", user.matrix_user_id),
            });
        }
        if taken_refs.contains(&user.reference_id) {
            errors.push(ImportRowError {
                row: user.row,
                message: format!("Reference ID {} is already stored.", user.reference_id),
            });
        }
    }
    if !errors.is_empty() {
        return Ok(ImportUsersResult {
            imported: 0,
            errors,
        });
    }

    let now = Utc::now();
    let records: Vec<UserRecord> = parsed
        .users
        .iter()
        .map(|user| UserRecord {
            id: Uuid::new_v4(),
            username: user.username.clone(),
            status: if user.active {
                UserStatus::Active.as_str()
            } else {
                UserStatus::Inactive.as_str()
            }
            .to_string(),
            matrix_user_id: Some(user.matrix_user_id.clone()),
            short_name: Some(user.short_name.clone()),
            full_name: user.full_name.clone(),
            reference_id: Some(user.reference_id),
            created_at: now,
            updated_at: now,
        })
        .collect();
    let imported = u32::try_from(records.len()).unwrap_or(u32::MAX);
    repo.insert_all(&records).await.map_err(map_db_error)?;
    tracing::info!(imported, "imported users from a spreadsheet");
    Ok(ImportUsersResult {
        imported,
        errors: Vec::new(),
    })
}

/// One person typed in by an operator. Same identity rules as a spreadsheet row.
pub async fn register_user(
    repo: &UserRepository,
    matrix_user_id: &str,
    username: &str,
    short_name: &str,
    full_name: &str,
    reference_id: &str,
    active: bool,
) -> Result<super::User, UserError> {
    let user = parse_registration(
        matrix_user_id,
        username,
        short_name,
        full_name,
        reference_id,
        active,
    )?;
    let occupied = repo
        .occupied_import_keys(
            std::slice::from_ref(&user.matrix_user_id),
            std::slice::from_ref(&user.reference_id),
        )
        .await
        .map_err(map_db_error)?;
    if occupied
        .iter()
        .any(|(id, _)| id.as_deref() == Some(user.matrix_user_id.as_str()))
    {
        return Err(UserError::DuplicateId);
    }
    if occupied
        .iter()
        .any(|(_, reference)| *reference == Some(user.reference_id))
    {
        return Err(UserError::DuplicateReference);
    }

    let now = Utc::now();
    let record = UserRecord {
        id: Uuid::new_v4(),
        username: user.username.clone(),
        status: if user.active {
            UserStatus::Active.as_str()
        } else {
            UserStatus::Inactive.as_str()
        }
        .to_string(),
        matrix_user_id: Some(user.matrix_user_id.clone()),
        short_name: Some(user.short_name.clone()),
        full_name: user.full_name.clone(),
        reference_id: Some(user.reference_id),
        created_at: now,
        updated_at: now,
    };
    repo.insert(&record).await.map_err(map_register_db_error)?;
    tracing::info!(user_id = %record.id, "registered user");
    Ok(super::User {
        id: record.id,
        username: record.username,
        status: if user.active {
            UserStatus::Active
        } else {
            UserStatus::Inactive
        },
        matrix_user_id: record.matrix_user_id,
        short_name: record.short_name,
        full_name: record.full_name,
        reference_id: record.reference_id,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn parse_registration(
    id: &str,
    name: &str,
    short: &str,
    full: &str,
    reference: &str,
    active: bool,
) -> Result<ImportedUser, UserError> {
    Ok(ImportedUser {
        row: 0,
        matrix_user_id: matrix_user_id(id).map_err(|_| UserError::InvalidId)?,
        username: normalize_username(name)?,
        short_name: short_name_value(short).map_err(|_| UserError::InvalidShortName)?,
        full_name: full_name_value(full).map_err(|_| UserError::InvalidFullName)?,
        reference_id: reference_id(reference).map_err(|_| UserError::InvalidReference)?,
        active,
    })
}

struct ParsedSheet {
    users: Vec<ImportedUser>,
    errors: Vec<ImportRowError>,
}

fn parse_user_rows(rows: &[Vec<String>]) -> ParsedSheet {
    let Some(header_index) = rows
        .iter()
        .position(|row| row.iter().any(|cell| !cell.trim().is_empty()))
    else {
        return ParsedSheet {
            users: Vec::new(),
            errors: vec![ImportRowError {
                row: 1,
                message: "The sheet has no column headings.".into(),
            }],
        };
    };
    let mut columns: HashMap<&str, usize> = HashMap::new();
    let mut errors = Vec::new();
    for (index, cell) in rows[header_index].iter().enumerate() {
        let Some(kind) = column_kind(cell) else {
            continue;
        };
        let key = kind_key(kind);
        if columns.contains_key(key) {
            errors.push(ImportRowError {
                row: (header_index + 1) as u32,
                message: format!("{key} is listed twice."),
            });
        } else {
            columns.insert(key, index);
        }
    }
    for required in ["id", "name", "short name", "reference id"] {
        if !columns.contains_key(required) {
            errors.push(ImportRowError {
                row: (header_index + 1) as u32,
                message: format!("Column {required} is required."),
            });
        }
    }
    if !errors.is_empty() {
        return ParsedSheet {
            users: Vec::new(),
            errors,
        };
    }

    let mut users = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut seen_refs = HashSet::new();
    for (offset, row) in rows.iter().enumerate().skip(header_index + 1) {
        let row_number = (offset + 1) as u32;
        let id = cell_at(row, columns.get("id").copied());
        let name = cell_at(row, columns.get("name").copied());
        let short_name = cell_at(row, columns.get("short name").copied());
        let reference = cell_at(row, columns.get("reference id").copied());
        let full_name = cell_at(row, columns.get("full name").copied());
        let active = cell_at(row, columns.get("active").copied());
        if id.is_empty()
            && name.is_empty()
            && short_name.is_empty()
            && reference.is_empty()
            && full_name.is_empty()
            && active.is_empty()
        {
            continue;
        }
        let mut row_failed = false;
        let matrix_user_id = match matrix_user_id(&id) {
            Ok(value) => value,
            Err(message) => {
                errors.push(ImportRowError {
                    row: row_number,
                    message,
                });
                row_failed = true;
                String::new()
            }
        };
        let username = match normalize_username(&name) {
            Ok(value) => value,
            Err(_) => {
                errors.push(ImportRowError {
                    row: row_number,
                    message: "Name is required and must be 200 characters or fewer.".into(),
                });
                row_failed = true;
                String::new()
            }
        };
        let short_name = match short_name_value(&short_name) {
            Ok(value) => value,
            Err(message) => {
                errors.push(ImportRowError {
                    row: row_number,
                    message,
                });
                row_failed = true;
                String::new()
            }
        };
        let reference_id = match reference_id(&reference) {
            Ok(value) => value,
            Err(message) => {
                errors.push(ImportRowError {
                    row: row_number,
                    message,
                });
                row_failed = true;
                0
            }
        };
        let full_name = match full_name_value(&full_name) {
            Ok(value) => value,
            Err(message) => {
                errors.push(ImportRowError {
                    row: row_number,
                    message,
                });
                row_failed = true;
                None
            }
        };
        let active = match active_value(&active) {
            Ok(value) => value,
            Err(message) => {
                errors.push(ImportRowError {
                    row: row_number,
                    message,
                });
                row_failed = true;
                true
            }
        };
        if row_failed {
            continue;
        }
        if !seen_ids.insert(matrix_user_id.clone()) {
            errors.push(ImportRowError {
                row: row_number,
                message: format!("ID {matrix_user_id} is repeated."),
            });
            continue;
        }
        if !seen_refs.insert(reference_id) {
            errors.push(ImportRowError {
                row: row_number,
                message: format!("Reference ID {reference_id} is repeated."),
            });
            continue;
        }
        users.push(ImportedUser {
            row: row_number,
            matrix_user_id,
            username,
            short_name,
            full_name,
            reference_id,
            active,
        });
    }
    ParsedSheet { users, errors }
}

fn kind_key(kind: Column) -> &'static str {
    match kind {
        Column::Id => "id",
        Column::Name => "name",
        Column::ShortName => "short name",
        Column::ReferenceId => "reference id",
        Column::FullName => "full name",
        Column::Active => "active",
    }
}

fn column_kind(header: &str) -> Option<Column> {
    match normalize_header(header).as_str() {
        "id" | "user id" => Some(Column::Id),
        "name" => Some(Column::Name),
        "short name" => Some(Column::ShortName),
        "reference id" | "ref id" => Some(Column::ReferenceId),
        "full name" => Some(Column::FullName),
        "active" => Some(Column::Active),
        _ => None,
    }
}

fn normalize_header(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn cell_at(row: &[String], index: Option<usize>) -> String {
    index
        .and_then(|index| row.get(index))
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn matrix_user_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > 15
        || !value.chars().all(|ch| ch.is_ascii_alphanumeric())
    {
        return Err("ID is required and must be 1–15 letters or digits.".into());
    }
    Ok(value.to_string())
}

fn short_name_value(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > MAX_SHORT_NAME {
        return Err("Short Name is required and must be 15 characters or fewer.".into());
    }
    Ok(value.to_string())
}

fn full_name_value(value: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_FULL_NAME {
        return Err("Full Name must be 200 characters or fewer.".into());
    }
    Ok(Some(value.to_string()))
}

fn reference_id(value: &str) -> Result<i64, String> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("Reference ID is required and must be a number from 1 to 99999999.".into());
    }
    let parsed: i64 = value.parse().map_err(|_| {
        "Reference ID is required and must be a number from 1 to 99999999.".to_string()
    })?;
    if !(1..=99_999_999).contains(&parsed) {
        return Err("Reference ID is required and must be a number from 1 to 99999999.".into());
    }
    Ok(parsed)
}

fn active_value(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "1" | "y" | "yes" | "true" | "active" => Ok(true),
        "0" | "n" | "no" | "false" | "inactive" => Ok(false),
        _ => Err("Active must be yes or no.".into()),
    }
}

fn read_sheet(bytes: &[u8]) -> Result<Vec<Vec<String>>, ()> {
    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor).map_err(|_| ())?;
    let range = workbook.worksheet_range_at(0).ok_or(())?.map_err(|_| ())?;
    let mut rows = Vec::new();
    for row in range.rows() {
        rows.push(row.iter().map(cell_text).collect());
    }
    Ok(rows)
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Int(value) => value.to_string(),
        Data::Float(value) if value.is_finite() && value.fract() == 0.0 => {
            format!("{}", *value as i64)
        }
        Data::Float(value) => value.to_string(),
        Data::String(value) => value.trim().to_string(),
        Data::Bool(value) => {
            if *value {
                "1".into()
            } else {
                "0".into()
            }
        }
        Data::Empty => String::new(),
        other => other.to_string().trim().to_string(),
    }
}

fn map_db_error(error: DatabaseError) -> UserError {
    tracing::error!(error = %error, "user import persistence failed");
    UserError::Unavailable
}

fn map_register_db_error(error: DatabaseError) -> UserError {
    match &error {
        DatabaseError::Query(sqlx::Error::Database(db))
            if db.code().as_deref() == Some("23505") =>
        {
            match db.constraint() {
                Some("users_matrix_user_id_unique") => UserError::DuplicateId,
                Some("users_reference_id_unique") => UserError::DuplicateReference,
                _ => {
                    tracing::error!(error = %error, "user registration hit an unexpected unique constraint");
                    UserError::Unavailable
                }
            }
        }
        _ => {
            tracing::error!(error = %error, "user registration persistence failed");
            UserError::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_registration, parse_user_rows};
    use crate::domains::users::UserError;

    fn sheet(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_string()).collect())
            .collect()
    }

    #[test]
    fn required_columns_and_blank_active() {
        let parsed = parse_user_rows(&sheet(&[
            &[
                "ID",
                "Name",
                "Short Name",
                "Reference ID",
                "Full Name",
                "Active",
            ],
            &["vsp099", "office", "office", "1245", "office", ""],
        ]));
        assert!(parsed.errors.is_empty());
        assert_eq!(parsed.users.len(), 1);
        assert_eq!(parsed.users[0].matrix_user_id, "vsp099");
        assert_eq!(parsed.users[0].username, "office");
        assert_eq!(parsed.users[0].short_name, "office");
        assert_eq!(parsed.users[0].full_name.as_deref(), Some("office"));
        assert_eq!(parsed.users[0].reference_id, 1245);
        assert!(parsed.users[0].active);
    }

    #[test]
    fn header_capitalization_is_ignored() {
        let parsed = parse_user_rows(&sheet(&[
            &[" id ", "NAME", "short   name", "Reference id"],
            &["Door1", "Hulk", "Hulk", "10000001"],
        ]));
        assert!(parsed.errors.is_empty());
        assert_eq!(parsed.users[0].matrix_user_id, "Door1");
        assert!(parsed.users[0].full_name.is_none());
    }

    #[test]
    fn missing_short_name_rejects_the_sheet() {
        let parsed = parse_user_rows(&sheet(&[
            &["ID", "Name", "Short Name", "Reference ID"],
            &["vsp099", "office", "", "1245"],
        ]));
        assert!(parsed.users.is_empty());
        assert!(parsed.errors.iter().any(|error| error.row == 2));
    }

    #[test]
    fn duplicate_reference_id_rejects_the_sheet() {
        let parsed = parse_user_rows(&sheet(&[
            &["ID", "Name", "Short Name", "Reference ID"],
            &["vsp099", "office", "office", "1245"],
            &["vsp100", "gate", "gate", "1245"],
        ]));
        assert_eq!(parsed.users.len(), 1);
        assert!(parsed
            .errors
            .iter()
            .any(|error| error.message.contains("repeated")));
    }

    #[test]
    fn manual_registration_accepts_the_same_fields_as_a_sheet_row() {
        let user = parse_registration("vsp099", " office ", "office", "", "1245", true).unwrap();
        assert_eq!(user.matrix_user_id, "vsp099");
        assert_eq!(user.username, "office");
        assert_eq!(user.short_name, "office");
        assert!(user.full_name.is_none());
        assert_eq!(user.reference_id, 1245);
        assert!(user.active);
    }

    #[test]
    fn manual_registration_rejects_a_blank_id_and_a_bad_reference() {
        assert_eq!(
            parse_registration("", "office", "office", "", "1245", true).unwrap_err(),
            UserError::InvalidId
        );
        assert_eq!(
            parse_registration("vsp099", "office", "office", "", "0", true).unwrap_err(),
            UserError::InvalidReference
        );
    }
}
