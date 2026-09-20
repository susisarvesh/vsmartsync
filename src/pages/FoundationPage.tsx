import { AppInfoCard } from "../components/AppInfoCard";
import { PostgresSetupCard } from "../components/PostgresSetupCard";
import { useAppInfo } from "../hooks/useAppInfo";
import { useDatabaseStatus } from "../hooks/useDatabaseStatus";

function isDesktopApp(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function FoundationPage() {
  const inDesktopApp = isDesktopApp();
  const { info, error, loading } = useAppInfo();
  const database = useDatabaseStatus();

  return (
    <main className="container">
      <h1>Vsmart Sync</h1>
      <p className="lede">
        Project foundation only. This screen verifies React → Tauri → Rust and
        local PostgreSQL.
      </p>
      {!inDesktopApp && (
        <p className="error" role="alert">
          Vsmart Sync is a local desktop app. Close this browser tab and run{" "}
          <code>npm install && npm start</code> from the project folder, then
          use the Vsmart Sync window.
        </p>
      )}
      {inDesktopApp && loading && <p>Loading application info from Rust…</p>}
      {inDesktopApp && error && (
        <p className="error" role="alert">
          Failed to invoke <code>get_app_info</code>: {error}
        </p>
      )}
      {inDesktopApp && info && <AppInfoCard info={info} />}
      {inDesktopApp && database.loading && (
        <p>Checking local PostgreSQL…</p>
      )}
      {inDesktopApp && database.error && (
        <p className="error" role="alert">
          Failed to check PostgreSQL: {database.error}
        </p>
      )}
      {inDesktopApp && database.status && (
        <PostgresSetupCard
          status={database.status}
          retrying={database.retrying}
          onRetry={database.retry}
        />
      )}
    </main>
  );
}
