import { invoke } from "@tauri-apps/api/core";

export type DatabaseEngineChoice = "postgres" | "mysql" | "mssql";

export type SetupStatus = {
  privacyAccepted: boolean;
  engine: DatabaseEngineChoice | null;
  ready: boolean;
  schemaReady: boolean;
  needsAdminLogin: boolean;
  message: string;
};

export async function getSetupStatus(): Promise<SetupStatus> {
  return invoke<SetupStatus>("get_setup_status");
}

export async function acceptDatabasePrivacy(): Promise<SetupStatus> {
  return invoke<SetupStatus>("accept_database_privacy");
}

export async function provisionDatabase(input: {
  engine: DatabaseEngineChoice;
  adminUser?: string;
  adminPassword?: string;
}): Promise<SetupStatus> {
  return invoke<SetupStatus>("provision_database", {
    engine: input.engine,
    adminUser: input.adminUser ?? null,
    adminPassword: input.adminPassword ?? null,
  });
}
