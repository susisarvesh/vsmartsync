import { invoke } from "@tauri-apps/api/core";
import type {
  CreateEnrollmentInput,
  DeviceEnrollmentOptions,
  Enrollment,
  ListEnrollmentsFilter,
} from "../types/enrollments";

/**
 * Frontend access to Rust must go through this services layer.
 * Enrollment is desired assignment only — no Matrix calls from React.
 */
export async function listEnrollments(
  filter: ListEnrollmentsFilter = {},
): Promise<Enrollment[]> {
  return invoke<Enrollment[]>("list_enrollments", {
    userId: filter.userId ?? null,
    deviceId: filter.deviceId ?? null,
    credentialId: filter.credentialId ?? null,
    status: filter.status ?? null,
    limit: filter.limit ?? null,
    offset: filter.offset ?? null,
  });
}

export async function getEnrollment(id: string): Promise<Enrollment> {
  return invoke<Enrollment>("get_enrollment", { id });
}

export async function createEnrollment(
  input: CreateEnrollmentInput,
): Promise<Enrollment> {
  return invoke<Enrollment>("create_enrollment", {
    userId: input.userId,
    credentialId: input.credentialId,
    deviceId: input.deviceId,
  });
}

export async function cancelEnrollment(id: string): Promise<Enrollment> {
  return invoke<Enrollment>("cancel_enrollment", { id });
}

export async function revokeEnrollment(id: string): Promise<Enrollment> {
  return invoke<Enrollment>("revoke_enrollment", { id });
}

export async function retryEnrollment(id: string): Promise<Enrollment> {
  return invoke<Enrollment>("retry_enrollment", { id });
}

export async function deviceEnrollmentOptions(
  deviceId: string,
): Promise<DeviceEnrollmentOptions> {
  return invoke<DeviceEnrollmentOptions>("device_enrollment_options", {
    deviceId,
  });
}

export async function enrollOnDevice(
  deviceId: string,
  userId: string,
  enrollType: string,
): Promise<Enrollment> {
  return invoke<Enrollment>("enroll_on_device", {
    deviceId,
    userId,
    enrollType,
  });
}

export function enrollmentErrorMessage(code: string): string {
  switch (code) {
    case "ENROLLMENT_NOT_FOUND":
      return "That enrollment does not exist.";
    case "ENROLLMENT_DUPLICATE":
      return "An open enrollment already exists for that credential and device.";
    case "ENROLLMENT_INVALID_TRANSITION":
      return "That status change is not allowed.";
    case "ENROLLMENT_USER_INACTIVE":
      return "Select an active user.";
    case "ENROLLMENT_CREDENTIAL_INACTIVE":
      return "Select an active credential.";
    case "ENROLLMENT_CREDENTIAL_OWNERSHIP":
      return "That credential does not belong to the selected user.";
    case "USER_NOT_FOUND":
      return "Select an existing user.";
    case "CREDENTIAL_NOT_FOUND":
      return "Select an existing credential.";
    case "DEVICE_NOT_FOUND":
      return "Select an existing device.";
    case "DEVICE_INACTIVE":
      return "Activate the device before enrollment.";
    case "USER_DEVICE_NOT_FOUND":
      return "Assign the user to this device before enrollment.";
    case "ENROLLMENT_UNSUPPORTED":
      return "This device did not report support for that enrollment type.";
    case "ENROLLMENT_NOT_CAPTURED":
      return "The device started enrollment, but the card or face was not saved on that user. Present it when the reader prompts, then try again.";
    case "ENROLLMENT_INVALID_TYPE":
      return "Choose an enrollment type the device supports.";
    case "DEVICE_SECRET_UNAVAILABLE":
      return "Could not access the saved device password.";
    case "DEVICE_SECRET_CORRUPT":
      return "The saved device password cannot be read. Choose Set Password on the device, then try again.";
    case "DEVICE_OFFLINE":
      return "Could not reach the device.";
    case "DEVICE_AUTH_FAILED":
      return "The device rejected the login.";
    case "DEVICE_BAD_RESPONSE":
      return "The device returned an unexpected response.";
    case "MATRIX_TIMEOUT":
      return "The device did not finish enrollment in time.";
    case "MATRIX_UNREACHABLE":
      return "The device could not be reached.";
    case "MATRIX_INVALID_ARGUMENT":
      return "That user could not be enrolled with the documented field limits.";
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    default:
      if (code === "MATRIX_API_ERROR:16") {
        return "The reader is on another screen, so the card was not saved. Leave that screen, start enrollment again, and present the card only when the reader asks.";
      }
      if (code.startsWith("MATRIX_API_ERROR:")) {
        return `The device refused enrollment (code ${code.slice("MATRIX_API_ERROR:".length)}).`;
      }
      return "Could not complete that enrollment action.";
  }
}
