import { invoke } from "@tauri-apps/api/core";
import type { DatabaseStatus } from "../types/database";

/**
 * Frontend access to Rust must go through this services layer.
 * Do not call PostgreSQL from React.
 */
export async function getDatabaseStatus(): Promise<DatabaseStatus> {
  return invoke<DatabaseStatus>("get_database_status");
}

export async function connectDatabase(): Promise<DatabaseStatus> {
  return invoke<DatabaseStatus>("connect_database");
}
