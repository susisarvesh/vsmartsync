import { invoke } from "@tauri-apps/api/core";
import type { User, UserDeviceAssignment, UserOnDevice } from "../types/users";

/**
 * Frontend access to Rust must go through this services layer.
 * Do not call PostgreSQL or Matrix devices from React.
 * Status changes only through activate and deactivate.
 */
export async function listUsers(): Promise<User[]> {
  return invoke<User[]>("list_users");
}

export async function createUser(username: string): Promise<User> {
  return invoke<User>("create_user", { username });
}

export async function updateUser(id: string, username: string): Promise<User> {
  return invoke<User>("update_user", { id, username });
}

export async function activateUser(id: string): Promise<User> {
  return invoke<User>("activate_user", { id });
}

export async function deactivateUser(id: string): Promise<User> {
  return invoke<User>("deactivate_user", { id });
}

export async function deleteUser(id: string): Promise<void> {
  return invoke("delete_user", { id });
}

export async function listUserDevices(
  userId: string,
): Promise<UserDeviceAssignment[]> {
  return invoke<UserDeviceAssignment[]>("list_user_devices", { userId });
}

export async function listUsersForDevice(
  deviceId: string,
): Promise<UserOnDevice[]> {
  return invoke<UserOnDevice[]>("list_users_for_device", { deviceId });
}

export async function assignUserDevice(
  userId: string,
  deviceId: string,
): Promise<UserDeviceAssignment> {
  return invoke<UserDeviceAssignment>("assign_user_device", { userId, deviceId });
}

export async function removeUserDevice(
  userId: string,
  deviceId: string,
): Promise<void> {
  return invoke("remove_user_device", { userId, deviceId });
}

export function userErrorMessage(code: string): string {
  switch (code) {
    case "USER_INVALID_USERNAME":
      return "Enter a username (1–200 characters).";
    case "USER_NOT_FOUND":
      return "That user does not exist.";
    case "DEVICE_NOT_FOUND":
      return "That device does not exist.";
    case "USER_DEVICE_DUPLICATE":
      return "That user is already assigned to this device.";
    case "USER_DEVICE_NOT_FOUND":
      return "That device assignment does not exist.";
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    case "DEVICE_SECRET_UNAVAILABLE":
      return "Could not access the saved device password.";
    case "DEVICE_SECRET_CORRUPT":
      return "The saved device password cannot be read. Choose Set Password on the device, then try again.";
    case "DEVICE_AUTH_FAILED":
      return "The device rejected the login, so the user was not deleted.";
    case "DEVICE_BAD_RESPONSE":
      return "The device returned an unexpected response, so the user was kept.";
    case "MATRIX_TIMEOUT":
      return "The device did not finish deleting the user, so the user was kept.";
    case "MATRIX_UNREACHABLE":
      return "The device could not be reached, so the user was kept.";
    case "MATRIX_INVALID_ARGUMENT":
      return "That user could not be deleted with the documented field limits.";
    default:
      if (code.startsWith("MATRIX_API_ERROR:")) {
        return `The device refused to delete the user (code ${code.slice("MATRIX_API_ERROR:".length)}), so the user was kept.`;
      }
      return "Could not complete that user action.";
  }
}
