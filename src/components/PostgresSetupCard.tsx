import type { DatabaseStatus } from "@/types/database";

type PostgresSetupDetailsProps = {
  status: DatabaseStatus;
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

/** Setup steps shown inside the database connection dialog when offline. */
export function PostgresSetupDetails({ status }: PostgresSetupDetailsProps) {
  const current = STEPS[status.platform] ?? STEPS.linux;
  const otherPlatforms = Object.entries(STEPS).filter(
    ([id]) => id !== status.platform,
  );

  return (
    <div className="rounded-md border border-border bg-muted/40 p-3">
      <h4 className="text-sm font-medium text-foreground">
        Set up local PostgreSQL
      </h4>
      <p className="mt-1 text-xs text-muted-foreground">
        Vsmart Sync uses PostgreSQL on this computer only. Do not use Docker.
      </p>

      <h5 className="mt-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {current.title}
      </h5>
      <ol className="mt-1.5 list-decimal space-y-1 pl-4 text-sm text-foreground">
        {current.commands.map((command) => (
          <li key={command}>
            <code className="rounded bg-muted px-1 py-0.5 text-xs">
              {command}
            </code>
          </li>
        ))}
      </ol>
      <p className="mt-2 text-xs text-muted-foreground">
        After PostgreSQL is running, <code>npm start</code> creates the{" "}
        <code>{status.database}</code> role and database when <code>psql</code>{" "}
        is available.
      </p>

      <details className="mt-3">
        <summary className="cursor-pointer text-xs font-medium text-foreground">
          Other operating systems
        </summary>
        {otherPlatforms.map(([id, steps]) => (
          <div key={id} className="mt-2">
            <h5 className="text-xs font-medium">{steps.title}</h5>
            <ol className="mt-1 list-decimal space-y-1 pl-4 text-sm">
              {steps.commands.map((command) => (
                <li key={command}>
                  <code className="rounded bg-muted px-1 py-0.5 text-xs">
                    {command}
                  </code>
                </li>
              ))}
            </ol>
          </div>
        ))}
      </details>
    </div>
  );
}

