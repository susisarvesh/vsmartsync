//! First-run database choice.
//!
//! The administrator picks one local engine. When that server is already
//! installed, this module creates the `vsmart_sync` database and applies the
//! schema. The administrator password is not stored. The application password
//! stays in the workstation setup file and never goes to the UI.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{connect_and_migrate, DatabaseConfig};

const SETUP_FILE: &str = "database.json";
const APP_DATABASE: &str = "vsmart_sync";
const APP_USER: &str = "vsmart_sync";

const MYSQL_SCHEMA: &str = include_str!("schema/mysql.sql");
const MSSQL_SCHEMA: &str = include_str!("schema/mssql.sql");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseEngine {
    Postgres,
    Mysql,
    Mssql,
}

impl DatabaseEngine {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" => Some(Self::Postgres),
            "mysql" => Some(Self::Mysql),
            "mssql" | "sqlserver" | "sql-server" => Some(Self::Mssql),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
            Self::Mssql => "mssql",
        }
    }

    fn default_port(self) -> u16 {
        match self {
            Self::Postgres => 5432,
            Self::Mysql => 3306,
            Self::Mssql => 1433,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::Mysql => "MySQL",
            Self::Mssql => "SQL Server",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupFile {
    privacy_accepted_at: Option<String>,
    engine: Option<DatabaseEngine>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    user: Option<String>,
    #[serde(default)]
    url: Option<String>,
    schema_ready: bool,
}

impl Default for SetupFile {
    fn default() -> Self {
        Self {
            privacy_accepted_at: None,
            engine: None,
            host: None,
            port: None,
            database: None,
            user: None,
            url: None,
            schema_ready: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub privacy_accepted: bool,
    pub engine: Option<String>,
    pub ready: bool,
    pub schema_ready: bool,
    pub needs_admin_login: bool,
    pub message: String,
}

#[derive(Debug)]
enum ConnectFailure {
    Absent,
    Auth,
    Other,
}

pub fn stored_postgres_config() -> Option<DatabaseConfig> {
    let stored = read_setup_file().ok()?;
    let url = stored.url?;
    DatabaseConfig::from_url(url).ok()
}

pub fn status() -> SetupStatus {
    let stored = read_setup_file().unwrap_or_default();
    status_from(&stored, None)
}

pub fn accept_privacy() -> Result<SetupStatus, String> {
    let mut stored = read_setup_file().unwrap_or_default();
    if stored.privacy_accepted_at.is_none() {
        stored.privacy_accepted_at = Some(chrono::Utc::now().to_rfc3339());
        write_setup_file(&stored)?;
    }
    Ok(status_from(&stored, None))
}

pub async fn provision(
    engine_name: &str,
    admin_user: Option<String>,
    admin_password: Option<String>,
) -> Result<(SetupStatus, Option<DatabaseConfig>), String> {
    let engine = DatabaseEngine::parse(engine_name)
        .ok_or_else(|| "Choose PostgreSQL, MySQL, or SQL Server.".to_string())?;
    let mut stored = read_setup_file().unwrap_or_default();
    if stored.privacy_accepted_at.is_none() {
        return Err("Accept the privacy notice before choosing a database.".into());
    }

    let admin_user = clean_optional(admin_user);
    let admin_password = clean_optional(admin_password);
    if let Some(user) = &admin_user {
        if !is_identifier(user) {
            return Err("The administrator login must be a simple name.".into());
        }
    }

    let outcome = match engine {
        DatabaseEngine::Postgres => {
            provision_postgres(admin_user.as_deref(), admin_password.as_deref()).await
        }
        DatabaseEngine::Mysql => {
            provision_mysql(admin_user.as_deref(), admin_password.as_deref()).await
        }
        DatabaseEngine::Mssql => {
            provision_mssql(admin_user.as_deref(), admin_password.as_deref()).await
        }
    };

    match outcome {
        Ok(Provisioned::Postgres(config)) => {
            stored.engine = Some(engine);
            stored.host = Some("127.0.0.1".into());
            stored.port = Some(engine.default_port());
            stored.database = Some(APP_DATABASE.into());
            stored.user = Some(APP_USER.into());
            stored.url = Some(config.url().to_string());
            stored.schema_ready = true;
            if let Some(target) = config.public_target() {
                stored.host = Some(target.host);
                stored.port = Some(target.port);
                stored.database = Some(target.database);
                stored.user = Some(target.user);
            }
            write_setup_file(&stored)?;
            Ok((status_from(&stored, None), Some(config)))
        }
        Ok(Provisioned::SchemaOnly) => {
            stored.engine = Some(engine);
            stored.host = Some("127.0.0.1".into());
            stored.port = Some(engine.default_port());
            stored.database = Some(APP_DATABASE.into());
            stored.user = Some(APP_USER.into());
            stored.schema_ready = true;
            write_setup_file(&stored)?;
            Ok((status_from(&stored, None), None))
        }
        Err(failure) => {
            let needs_admin = matches!(failure, ConnectFailure::Auth);
            let message = match failure {
                ConnectFailure::Absent => format!(
                    "{} is not running on this computer. Install it locally, start the service, then try again.",
                    engine.label()
                ),
                ConnectFailure::Auth => format!(
                    "{} is installed, but the administrator login was refused. Enter the server administrator once. That password is not saved.",
                    engine.label()
                ),
                ConnectFailure::Other => format!(
                    "{} could not be prepared automatically. Check that the local service is running, then try again.",
                    engine.label()
                ),
            };
            Ok((
                SetupStatus {
                    privacy_accepted: true,
                    engine: Some(engine.as_str().to_string()),
                    ready: false,
                    schema_ready: false,
                    needs_admin_login: needs_admin,
                    message,
                },
                None,
            ))
        }
    }
}

enum Provisioned {
    Postgres(DatabaseConfig),
    SchemaOnly,
}

async fn provision_postgres(
    admin_user: Option<&str>,
    admin_password: Option<&str>,
) -> Result<Provisioned, ConnectFailure> {
    if let Ok(config) = DatabaseConfig::from_env() {
        if connect_and_migrate(&config).await.is_ok() {
            return Ok(Provisioned::Postgres(config));
        }
    }

    let password = generated_password();
    let admin = open_postgres_admin(admin_user, admin_password).await?;
    ensure_postgres_role_and_database(&admin, &password).await?;
    let config = DatabaseConfig::from_parts("127.0.0.1", 5432, APP_DATABASE, APP_USER, &password)
        .map_err(|_| ConnectFailure::Other)?;
    connect_and_migrate(&config)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    Ok(Provisioned::Postgres(config))
}

async fn open_postgres_admin(
    admin_user: Option<&str>,
    admin_password: Option<&str>,
) -> Result<sqlx::PgPool, ConnectFailure> {
    let mut urls = Vec::new();
    if let (Some(user), Some(password)) = (admin_user, admin_password) {
        urls.push(format!(
            "postgres://{}:{}@127.0.0.1:5432/postgres",
            encode_userinfo(user),
            encode_userinfo(password)
        ));
    } else {
        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            if is_identifier(&user) {
                urls.push(format!("postgres://{user}@127.0.0.1:5432/postgres"));
            }
        }
        urls.push("postgres://postgres@127.0.0.1:5432/postgres".to_string());
    }

    let mut saw_auth = false;
    for url in urls {
        match connect_pg(&url).await {
            Ok(pool) => return Ok(pool),
            Err(ConnectFailure::Auth) => saw_auth = true,
            Err(ConnectFailure::Absent) => {}
            Err(ConnectFailure::Other) => {}
        }
    }
    Err(if saw_auth {
        ConnectFailure::Auth
    } else {
        ConnectFailure::Absent
    })
}

async fn connect_pg(url: &str) -> Result<sqlx::PgPool, ConnectFailure> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .connect(url)
        .await
        .map_err(classify_connect)
}

async fn ensure_postgres_role_and_database(
    admin: &sqlx::PgPool,
    password: &str,
) -> Result<(), ConnectFailure> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = $1)")
        .bind(APP_USER)
        .fetch_one(admin)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    if !exists {
        let sql = format!(
            "CREATE ROLE {APP_USER} LOGIN PASSWORD '{}'",
            password.replace('\'', "''")
        );
        sqlx::query(&sql)
            .execute(admin)
            .await
            .map_err(|_| ConnectFailure::Other)?;
    }
    let db_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(APP_DATABASE)
            .fetch_one(admin)
            .await
            .map_err(|_| ConnectFailure::Other)?;
    if !db_exists {
        sqlx::query(&format!(
            "CREATE DATABASE {APP_DATABASE} OWNER {APP_USER}"
        ))
        .execute(admin)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    }
    Ok(())
}

async fn provision_mysql(
    admin_user: Option<&str>,
    admin_password: Option<&str>,
) -> Result<Provisioned, ConnectFailure> {
    let admin = open_mysql_admin(admin_user, admin_password).await?;
    let password = generated_password();
    let create_db = format!("CREATE DATABASE IF NOT EXISTS {APP_DATABASE}");
    sqlx::query(&create_db)
        .execute(&admin)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    let create_user = format!(
        "CREATE USER IF NOT EXISTS '{APP_USER}'@'127.0.0.1' IDENTIFIED BY '{}'",
        password.replace('\'', "''")
    );
    sqlx::query(&create_user)
        .execute(&admin)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    let grant = format!("GRANT ALL PRIVILEGES ON {APP_DATABASE}.* TO '{APP_USER}'@'127.0.0.1'");
    sqlx::query(&grant)
        .execute(&admin)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    let app_url = format!(
        "mysql://{APP_USER}:{}@127.0.0.1:3306/{APP_DATABASE}",
        encode_userinfo(&password)
    );
    let app = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&app_url)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    apply_script_if_missing(&app, MYSQL_SCHEMA, "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'users'")
        .await?;
    Ok(Provisioned::SchemaOnly)
}

async fn open_mysql_admin(
    admin_user: Option<&str>,
    admin_password: Option<&str>,
) -> Result<sqlx::MySqlPool, ConnectFailure> {
    let urls = if let (Some(user), Some(password)) = (admin_user, admin_password) {
        vec![format!(
            "mysql://{}:{}@127.0.0.1:3306/mysql",
            encode_userinfo(user),
            encode_userinfo(password)
        )]
    } else {
        vec![
            "mysql://root@127.0.0.1:3306/mysql".to_string(),
            "mysql://root:@127.0.0.1:3306/mysql".to_string(),
        ]
    };
    let mut saw_auth = false;
    for url in urls {
        match sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(error) => match classify_connect(error) {
                ConnectFailure::Auth => saw_auth = true,
                ConnectFailure::Absent | ConnectFailure::Other => {}
            },
        }
    }
    Err(if saw_auth {
        ConnectFailure::Auth
    } else {
        ConnectFailure::Absent
    })
}

async fn provision_mssql(
    admin_user: Option<&str>,
    admin_password: Option<&str>,
) -> Result<Provisioned, ConnectFailure> {
    let mut admin = open_mssql(admin_user, admin_password, "master").await?;
    let exists = mssql_database_exists(&mut admin).await?;
    if !exists {
        mssql_execute(&mut admin, "CREATE DATABASE vsmart_sync").await?;
    }
    drop(admin);
    let mut app = open_mssql(admin_user, admin_password, APP_DATABASE).await?;
    apply_mssql_script_if_missing(&mut app).await?;
    Ok(Provisioned::SchemaOnly)
}

async fn open_mssql(
    admin_user: Option<&str>,
    admin_password: Option<&str>,
    database: &str,
) -> Result<tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>, ConnectFailure> {
    let mut config = tiberius::Config::new();
    config.host("127.0.0.1");
    config.port(1433);
    config.database(database);
    config.trust_cert();
    let probe = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(("127.0.0.1", 1433)),
    )
    .await;
    if probe.is_err() || probe.ok().and_then(Result::ok).is_none() {
        return Err(ConnectFailure::Absent);
    }
    match (admin_user, admin_password) {
        (Some(user), Some(password)) => {
            config.authentication(tiberius::AuthMethod::sql_server(user, password));
        }
        _ => return Err(ConnectFailure::Auth),
    }
    let addr = config.get_addr();
    let tcp = tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(addr))
        .await
        .map_err(|_| ConnectFailure::Absent)?
        .map_err(|_| ConnectFailure::Absent)?;
    tcp.set_nodelay(true).map_err(|_| ConnectFailure::Other)?;
    tiberius::Client::connect(config, tokio_util::compat::TokioAsyncWriteCompatExt::compat_write(tcp))
        .await
        .map_err(|error| classify_tiberius(error.to_string()))
}

async fn mssql_database_exists(
    client: &mut tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>,
) -> Result<bool, ConnectFailure> {
    let stream = client
        .query(
            "SELECT CASE WHEN DB_ID('vsmart_sync') IS NULL THEN 0 ELSE 1 END",
            &[],
        )
        .await
        .map_err(|_| ConnectFailure::Other)?;
    let row = stream
        .into_row()
        .await
        .map_err(|_| ConnectFailure::Other)?;
    Ok(row.and_then(|row| row.get::<i32, _>(0)).unwrap_or(0) == 1)
}

async fn mssql_execute(
    client: &mut tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>,
    statement: &str,
) -> Result<(), ConnectFailure> {
    client
        .simple_query(statement)
        .await
        .map_err(|_| ConnectFailure::Other)?
        .into_results()
        .await
        .map_err(|_| ConnectFailure::Other)?;
    Ok(())
}

fn classify_tiberius(text: String) -> ConnectFailure {
    let text = text.to_ascii_lowercase();
    if text.contains("login") || text.contains("password") || text.contains("authentication") {
        ConnectFailure::Auth
    } else if text.contains("refused") || text.contains("timed out") || text.contains("timeout") {
        ConnectFailure::Absent
    } else {
        ConnectFailure::Other
    }
}

async fn apply_script_if_missing(
    pool: &sqlx::MySqlPool,
    script: &str,
    exists_sql: &str,
) -> Result<(), ConnectFailure> {
    let count: i64 = sqlx::query_scalar(exists_sql)
        .fetch_one(pool)
        .await
        .map_err(|_| ConnectFailure::Other)?;
    if count > 0 {
        return Ok(());
    }
    for statement in sql_statements(script) {
        sqlx::query(statement)
            .execute(pool)
            .await
            .map_err(|_| ConnectFailure::Other)?;
    }
    Ok(())
}

async fn apply_mssql_script_if_missing(
    client: &mut tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>,
) -> Result<(), ConnectFailure> {
    let stream = client
        .query(
            "SELECT COUNT(*) FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_NAME = 'users'",
            &[],
        )
        .await
        .map_err(|_| ConnectFailure::Other)?;
    let count = stream
        .into_row()
        .await
        .map_err(|_| ConnectFailure::Other)?
        .and_then(|row| row.get::<i32, _>(0))
        .unwrap_or(0);
    if count > 0 {
        return Ok(());
    }
    for statement in sql_statements(MSSQL_SCHEMA) {
        mssql_execute(client, statement).await?;
    }
    Ok(())
}

fn classify_connect(error: sqlx::Error) -> ConnectFailure {
    let text = error.to_string().to_ascii_lowercase();
    if text.contains("password")
        || text.contains("authentication")
        || text.contains("access denied")
        || text.contains("login failed")
    {
        ConnectFailure::Auth
    } else if text.contains("connection refused")
        || text.contains("timed out")
        || text.contains("timeout")
        || text.contains("could not connect")
        || text.contains("no such host")
        || text.contains("network")
    {
        ConnectFailure::Absent
    } else {
        ConnectFailure::Other
    }
}

fn status_from(stored: &SetupFile, message: Option<String>) -> SetupStatus {
    let privacy_accepted = stored.privacy_accepted_at.is_some();
    let engine = stored.engine.map(DatabaseEngine::as_str);
    let postgres_env_ready = DatabaseConfig::from_env().is_ok();
    let ready = privacy_accepted
        && engine == Some("postgres")
        && (stored.schema_ready || postgres_env_ready);
    let message = message.unwrap_or_else(|| {
        if !privacy_accepted {
            "Review the privacy notice to continue.".to_string()
        } else if stored.schema_ready && engine == Some("postgres") {
            "PostgreSQL is ready on this computer.".to_string()
        } else if stored.schema_ready {
            "The database and tables are created. Application screens still run on PostgreSQL statements, so choose PostgreSQL to use Vsmart Sync."
                .to_string()
        } else if privacy_accepted {
            "Choose the database already installed on this computer.".to_string()
        } else {
            String::new()
        }
    });
    SetupStatus {
        privacy_accepted,
        engine: engine.map(str::to_string),
        ready,
        schema_ready: stored.schema_ready,
        needs_admin_login: false,
        message,
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    value.len() <= 63
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub fn sql_statements(script: &str) -> Vec<&str> {
    script
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty() && !statement.starts_with("--"))
        .collect()
}

fn generated_password() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut bytes = [0u8; 24];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    bytes
        .into_iter()
        .map(|byte| ALPHABET[(byte as usize) % ALPHABET.len()] as char)
        .collect()
}

fn encode_userinfo(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn setup_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "The workstation home directory is not available.".to_string())?;
    Ok(PathBuf::from(home).join(".vsmart-sync").join(SETUP_FILE))
}

fn read_setup_file() -> Result<SetupFile, String> {
    let path = setup_path()?;
    if !path.exists() {
        return Ok(SetupFile::default());
    }
    let bytes = fs::read(&path).map_err(|_| "The setup file could not be read.".to_string())?;
    serde_json::from_slice(&bytes).map_err(|_| "The setup file could not be read.".to_string())
}

fn write_setup_file(value: &SetupFile) -> Result<(), String> {
    let path = setup_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|_| "The setup folder could not be created.".to_string())?;
        restrict_dir(dir);
    }
    let json = serde_json::to_vec_pretty(value)
        .map_err(|_| "The setup file could not be written.".to_string())?;
    write_private_file(&path, &json)
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "The setup file could not be written.".to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "The setup file could not be written.".to_string())
}

fn restrict_dir(dir: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
}

#[cfg(test)]
mod tests {
    use super::{is_identifier, sql_statements, MYSQL_SCHEMA, MSSQL_SCHEMA};

    #[test]
    fn identifiers_reject_sql_fragments() {
        assert!(is_identifier("vsmart_sync"));
        assert!(!is_identifier("vsmart;drop"));
        assert!(!is_identifier("vsmart sync"));
        assert!(!is_identifier(""));
    }

    #[test]
    fn schemas_create_the_application_tables() {
        for script in [MYSQL_SCHEMA, MSSQL_SCHEMA] {
            let statements = sql_statements(script);
            assert!(statements.len() > 8);
            let joined = statements.join("\n");
            for table in [
                "users",
                "devices",
                "credentials",
                "enrollments",
                "device_users",
                "user_devices",
                "enrollment_sessions",
                "events",
            ] {
                assert!(
                    joined.contains(&format!("CREATE TABLE")) && joined.contains(table),
                    "{table} missing"
                );
            }
        }
    }

    #[test]
    fn setup_status_json_has_no_password_field() {
        let status = super::SetupStatus {
            privacy_accepted: true,
            engine: Some("mysql".into()),
            ready: false,
            schema_ready: true,
            needs_admin_login: false,
            message: "tables created".into(),
        };
        let json = serde_json::to_string(&status).expect("json");
        assert!(!json.contains("password"));
        assert!(!json.contains("mysql://"));
    }
}
