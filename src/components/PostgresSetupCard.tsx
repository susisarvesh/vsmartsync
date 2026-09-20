import type { DatabaseStatus } from "../types/database";

type PostgresSetupCardProps = {
  status: DatabaseStatus;
  retrying: boolean;
  onRetry: () => void;
};

const STEPS: Record<string, { title: string; commands: string[] }> = {
  macos: {
    title: "macOS",
    commands: [
      "brew install postgresql@16",
      "brew services start postgresql@16",
      "npm install && npm start",
    ],
  },
  windows: {
    title: "Windows",
    commands: [
      "Install PostgreSQL from https://www.postgresql.org/download/windows/ (port 5432)",
      "Tick “command line tools” so psql is on PATH",
      "Start the postgresql Windows service",
      "npm install && npm start",
    ],
  },
  linux: {
    title: "Linux (Debian/Ubuntu)",
    commands: [
      "sudo apt install postgresql postgresql-contrib",
      "sudo systemctl enable --now postgresql",
      "npm install && npm start",
    ],
  },
};

export function PostgresSetupCard({
  status,
  retrying,
  onRetry,
}: PostgresSetupCardProps) {
  const current = STEPS[status.platform] ?? STEPS.linux;
  const otherPlatforms = Object.entries(STEPS).filter(
    ([id]) => id !== status.platform,
  );

  if (status.connected) {
    return (
      <section className="setup-card setup-card-ok" aria-live="polite">
        <h2>PostgreSQL</h2>
        <p className="setup-ok">{status.message}</p>
        <p className="setup-meta">
          {status.user}@{status.host}:{status.port}/{status.database}
        </p>
      </section>
    );
  }

  return (
    <section className="setup-card" aria-live="polite">
      <h2>Set up local PostgreSQL</h2>
      <p className="error" role="alert">
        {status.message}
      </p>
      <p className="lede">
        Vsmart Sync uses PostgreSQL on this computer only. Do not use Docker.
        Target:{" "}
        <code>
          {status.host}:{status.port}/{status.database}
        </code>{" "}
        (user <code>{status.user}</code>).
      </p>

      <h3>{current.title}</h3>
      <ol className="setup-steps">
        {current.commands.map((command) => (
          <li key={command}>
            <code>{command}</code>
          </li>
        ))}
      </ol>
      <p className="lede">
        After PostgreSQL is running, <code>npm start</code> creates the{" "}
        <code>{status.database}</code> role and database when <code>psql</code>{" "}
        is available. Then click Try again.
      </p>
      <button type="button" onClick={onRetry} disabled={retrying}>
        {retrying ? "Connecting…" : "Try again"}
      </button>

      <details className="setup-other">
        <summary>Other operating systems</summary>
        {otherPlatforms.map(([id, steps]) => (
          <div key={id}>
            <h3>{steps.title}</h3>
            <ol className="setup-steps">
              {steps.commands.map((command) => (
                <li key={command}>
                  <code>{command}</code>
                </li>
              ))}
            </ol>
          </div>
        ))}
      </details>
    </section>
  );
}
