import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "../types/app";

/**
 * Frontend access to Rust must go through this services layer.
 * Do not call PostgreSQL or Matrix devices from React.
 */
export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}
