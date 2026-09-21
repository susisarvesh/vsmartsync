import { invoke } from "@tauri-apps/api/core";
import type {
  CreateEnrollmentInput,
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
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    default:
      return "Could not complete that enrollment action.";
  }
}
