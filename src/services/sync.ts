import { invoke } from "@tauri-apps/api/core";

export type SyncedDeviceUser = {
  userId: string;
  username: string;
  matrixUserId: string;
  matrixRefUserId: number;
  provisionedAt: string;
};

export type SyncUserFailure = {
  userId: string;
  username: string | null;
  code: string;
};

export type SyncUsersResult = {
  deviceId: string;
  synced: SyncedDeviceUser[];
  failed: SyncUserFailure[];
};

/**
 * Push users that are already assigned to a device onto that COSEC device.
 * Does not send credentials or device passwords to the UI.
 */
export async function syncAssignedUsers(
  deviceId: string,
  userIds: string[],
): Promise<SyncUsersResult> {
  return invoke<SyncUsersResult>("sync_assigned_users", { deviceId, userIds });
}

export function syncErrorMessage(code: string): string {
  if (code.startsWith("MATRIX_API_ERROR:")) {
    const deviceCode = code.slice("MATRIX_API_ERROR:".length);
    return `The device refused the user (code ${deviceCode}).`;
  }
  switch (code) {
    case "DEVICE_NOT_FOUND":
      return "That device does not exist.";
    case "DEVICE_INACTIVE":
      return "Activate the device before adding users on it.";
    case "DEVICE_INVALID_PORT":
      return "That device port cannot be used.";
    case "DEVICE_SECRET_UNAVAILABLE":
      return "Could not access the secure credential store.";
    case "DEVICE_SECRET_CORRUPT":
      return "The saved device password cannot be read. Choose Set Password on the device, enter it again, then sync.";
    case "SYNC_NO_USERS":
      return "Select at least one assigned user.";
    case "SYNC_TOO_MANY":
      return "Too many users were selected for one sync.";
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    case "USER_NOT_FOUND":
      return "That user does not exist.";
    case "USER_DEVICE_NOT_FOUND":
      return "That user is not assigned to this device.";
    case "DEVICE_USER_SEQUENCE_EXHAUSTED":
      return "This device has no remaining Matrix user ids.";
    case "MATRIX_TIMEOUT":
      return "The device did not respond in time.";
    case "MATRIX_UNREACHABLE":
      return "The device could not be reached.";
    case "MATRIX_AUTH_FAILED":
      return "The device rejected the login.";
    case "MATRIX_BAD_RESPONSE":
      return "The device returned an unexpected response.";
    case "MATRIX_INVALID_TARGET":
      return "The device address could not be used.";
    case "MATRIX_INVALID_ARGUMENT":
      return "The user could not be sent with the documented field limits.";
    default:
      return "Could not add that user on the device.";
  }
}
