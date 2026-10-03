import { invoke } from "@tauri-apps/api/core";
import type {
  CardRead,
  CardReaderStatus,
  CardTestResult,
  CreateEnrollmentInput,
  DeviceEnrollmentOptions,
  Enrollment,
  EnrollmentSession,
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

export async function startDeviceEnrollment(
  deviceId: string,
  userId: string,
  enrollType: string,
): Promise<EnrollmentSession> {
  return invoke<EnrollmentSession>("start_device_enrollment", {
    deviceId,
    userId,
    enrollType,
  });
}

export async function getDeviceEnrollmentSession(
  id: string,
): Promise<EnrollmentSession> {
  return invoke<EnrollmentSession>("get_device_enrollment_session", { id });
}

export async function cancelDeviceEnrollmentSession(
  id: string,
): Promise<EnrollmentSession> {
  return invoke<EnrollmentSession>("cancel_device_enrollment_session", { id });
}

export async function readCard(deviceId: string): Promise<CardRead> {
  return invoke<CardRead>("read_card", { deviceId });
}

export async function getCardReaderStatus(
  deviceId: string,
): Promise<CardReaderStatus> {
  return invoke<CardReaderStatus>("get_card_reader_status", { deviceId });
}

export async function testCard(deviceId: string): Promise<CardTestResult> {
  return invoke<CardTestResult>("test_card", { deviceId });
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
      return "The device started enrollment, but the credential count did not increase. Present it when the reader prompts, then try again.";
    case "ENROLLMENT_DEVICE_BUSY":
      return "This device is already enrolling someone. Wait for that attempt to finish, then try again.";
    case "ENROLLMENT_PERSISTENCE_FAILED":
      return "The device enrolled the credential, but it could not be saved here. Do not enroll again until this is reconciled.";
    case "ENROLLMENT_SESSION_NOT_FOUND":
      return "That enrollment session does not exist.";
    case "ENROLLMENT_CANCELLED":
      return "Enrollment was cancelled in this app. The reader was not remotely cancelled.";
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
      return "Enrollment timed out. Please try again.";
    case "MATRIX_UNREACHABLE":
      return "The device could not be reached.";
    case "MATRIX_INVALID_ARGUMENT":
      return "That user could not be enrolled with the documented field limits.";
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    default:
      if (code === "MATRIX_API_ERROR:16") {
        return "The Matrix device is currently busy with another operation.";
      }
      if (code === "MATRIX_API_ERROR:26") {
        return "The card read parameters do not apply to this card type.";
      }
      if (code === "MATRIX_API_ERROR:27") {
        return "Card was not detected before the enrollment/read timeout.";
      }
      if (code === "MATRIX_API_ERROR:28") {
        return "The device detected a card but could not read it. Check card placement, card technology, and card configuration.";
      }
      if (code === "MATRIX_API_ERROR:29") {
        return "Wrong card type. The card does not match the reader configured on this device.";
      }
      if (code === "MATRIX_API_ERROR:30") {
        return "The card could not be read because its configured security key does not match the device configuration.";
      }
      if (code.startsWith("MATRIX_API_ERROR:")) {
        return `The device refused enrollment (code ${code.slice("MATRIX_API_ERROR:".length)}).`;
      }
      return "Could not complete that enrollment action.";
  }
}
