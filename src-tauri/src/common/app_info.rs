use serde::{Deserialize, Serialize};

/// Public, non-sensitive application metadata returned to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub stage: String,
}

pub fn app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: env!("CARGO_PKG_DESCRIPTION").to_string(),
        stage: "foundation".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::app_info;

    #[test]
    fn app_info_uses_package_metadata() {
        let info = app_info();
        assert_eq!(info.name, "matrixcosec");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.stage, "foundation");
    }

    #[test]
    fn app_info_round_trips_through_json() {
        let info = app_info();
        let json = serde_json::to_string(&info).expect("serialize");
        let parsed: super::AppInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, info);
    }
}
