import { invoke } from "@tauri-apps/api/core";
import type { User } from "../types/users";

/**
 * Frontend access to Rust must go through this services layer.
 * Do not call PostgreSQL or Matrix devices from React.
 * Do not set user status from the UI — deactivate is a domain operation.
 */
export async function listUsers(): Promise<User[]> {
  return invoke<User[]>("list_users");
}

export async function createUser(name: string): Promise<User> {
  return invoke<User>("create_user", { name });
}

export async function updateUser(id: string, name: string): Promise<User> {
  return invoke<User>("update_user", { id, name });
}

export async function deactivateUser(id: string): Promise<User> {
  return invoke<User>("deactivate_user", { id });
}

export function userErrorMessage(code: string): string {
  switch (code) {
    case "USER_INVALID_NAME":
      return "Enter a name (1–200 characters).";
    case "USER_NOT_FOUND":
      return "That user does not exist.";
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    default:
      return "Could not complete that user action.";
  }
}
