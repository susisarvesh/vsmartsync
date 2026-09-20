import { invoke } from "@tauri-apps/api/core";
import type {
  CreateCredentialInput,
  Credential,
  ListCredentialsFilter,
} from "../types/credentials";

/**
 * Frontend access to Rust must go through this services layer.
 * Never request plaintext PINs, card numbers, or ciphertext.
 */
export async function listCredentials(
  filter: ListCredentialsFilter = {},
): Promise<Credential[]> {
  return invoke<Credential[]>("list_credentials", {
    userId: filter.userId ?? null,
    credentialType: filter.credentialType ?? null,
    status: filter.status ?? null,
  });
}

export async function getCredential(id: string): Promise<Credential> {
  return invoke<Credential>("get_credential", { id });
}

export async function createCredential(
  input: CreateCredentialInput,
): Promise<Credential> {
  return invoke<Credential>("create_credential", {
    userId: input.userId,
    credentialType: input.credentialType,
    value: input.value,
  });
}

export async function updateCredential(
  id: string,
  value: string,
): Promise<Credential> {
  return invoke<Credential>("update_credential", { id, value });
}

export async function setCredentialStatus(
  id: string,
  status: "active" | "inactive",
): Promise<Credential> {
  return invoke<Credential>("set_credential_status", { id, status });
}

export function credentialErrorMessage(code: string): string {
  switch (code) {
    case "CREDENTIAL_INVALID_TYPE":
      return "Choose Card or PIN.";
    case "CREDENTIAL_INVALID_VALUE":
      return "Enter a valid credential value.";
    case "CREDENTIAL_INVALID_STATUS":
      return "Choose Active or Inactive.";
    case "CREDENTIAL_DUPLICATE":
      return "That credential already exists.";
    case "CREDENTIAL_NOT_FOUND":
      return "That credential does not exist.";
    case "USER_NOT_FOUND":
      return "Select an existing user.";
    case "CREDENTIAL_SECRET_UNAVAILABLE":
      return "Could not access the secure credential store.";
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    default:
      return "Could not complete that credential action.";
  }
}
