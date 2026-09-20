#!/usr/bin/env node
/**
 * Cross-platform first-run setup. Used by `npm start` on macOS, Windows, and Linux.
 * Copies `.env` if missing and, when `psql` is available, creates the local database.
 */

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const envPath = join(root, ".env");
const examplePath = join(root, ".env.example");

if (!existsSync(examplePath)) {
  console.error("Missing .env.example");
  process.exit(1);
}

if (!existsSync(envPath)) {
  copyFileSync(examplePath, envPath);
  console.log("Created .env from .env.example");
}

const ident = /^[A-Za-z_][A-Za-z0-9_]*$/;
const env = Object.fromEntries(
  readFileSync(envPath, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("#") && line.includes("="))
    .map((line) => {
      const eq = line.indexOf("=");
      return [line.slice(0, eq), line.slice(eq + 1)];
    }),
);

const db = env.POSTGRES_DB || "vsmart_sync";
const user = env.POSTGRES_USER || "vsmart_sync";
const password = env.POSTGRES_PASSWORD || "change-me";

if (!ident.test(db) || !ident.test(user)) {
  console.error("POSTGRES_DB and POSTGRES_USER must be simple identifiers");
  process.exit(1);
}

const psql = spawnSync(
  "psql",
  ["-d", "postgres", "-v", "ON_ERROR_STOP=1", "-tAc", "SELECT 1"],
  { encoding: "utf8" },
);

if (psql.error || psql.status !== 0) {
  console.log(
    "PostgreSQL: start a local server on 127.0.0.1:5432 (no Docker). psql was not usable yet.",
  );
  process.exit(0);
}

const escapedPassword = password.replaceAll("'", "''");
const ensureRole = `
DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = '${user}') THEN
    CREATE ROLE ${user} LOGIN PASSWORD '${escapedPassword}';
  END IF;
END
$$;
`;

const role = spawnSync("psql", ["-d", "postgres", "-v", "ON_ERROR_STOP=1", "-c", ensureRole], {
  encoding: "utf8",
});
if (role.status !== 0) {
  console.log("PostgreSQL role was not created automatically. Check local server access.");
  process.exit(0);
}

const exists = spawnSync(
  "psql",
  ["-d", "postgres", "-tAc", `SELECT 1 FROM pg_database WHERE datname = '${db}'`],
  { encoding: "utf8" },
);

if (exists.stdout.trim() !== "1") {
  const createdb = spawnSync(
    "psql",
    ["-d", "postgres", "-v", "ON_ERROR_STOP=1", "-c", `CREATE DATABASE ${db} OWNER ${user}`],
    { encoding: "utf8" },
  );
  if (createdb.status !== 0) {
    console.log("PostgreSQL database was not created automatically.");
  }
}
