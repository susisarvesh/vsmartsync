import { Button } from "@/components/ui/button";
import type { DatabaseStatus } from "@/types/database";

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
      <section className="rounded-lg border border-green-200 bg-green-50 p-4">
        <h3 className="text-sm font-semibold text-success">PostgreSQL</h3>
        <p className="mt-1 text-sm text-success">{status.message}</p>
        <p className="mt-1 text-xs text-muted-foreground">
          {status.user}@{status.host}:{status.port}/{status.database}
        </p>
      </section>
    );
  }

  return (
    <section className="rounded-lg border border-border bg-card p-4">
      <h3 className="text-sm font-semibold text-foreground">
        Set up local PostgreSQL
      </h3>
      <p className="mt-2 text-sm text-destructive" role="alert">
        {status.message}
      </p>
      <p className="mt-2 text-sm text-muted-foreground">
        Vsmart Sync uses PostgreSQL on this computer only. Do not use Docker.
        Target:{" "}
        <code className="rounded bg-muted px-1 py-0.5 text-xs">
          {status.host}:{status.port}/{status.database}
        </code>{" "}
        (user{" "}
        <code className="rounded bg-muted px-1 py-0.5 text-xs">
          {status.user}
        </code>
        ).
      </p>

      <h4 className="mt-4 text-sm font-medium">{current.title}</h4>
      <ol className="mt-2 list-decimal space-y-1 pl-5 text-sm text-foreground">
        {current.commands.map((command) => (
          <li key={command}>
            <code className="rounded bg-muted px-1 py-0.5 text-xs">
              {command}
            </code>
          </li>
        ))}
      </ol>
      <p className="mt-3 text-sm text-muted-foreground">
        After PostgreSQL is running, <code>npm start</code> creates the{" "}
        <code>{status.database}</code> role and database when <code>psql</code>{" "}
        is available. Then click Try again.
      </p>
      <Button
        type="button"
        className="mt-3"
        onClick={onRetry}
        disabled={retrying}
      >
        {retrying ? "Connecting…" : "Try again"}
      </Button>

      <details className="mt-4">
        <summary className="cursor-pointer text-sm font-medium text-foreground">
          Other operating systems
        </summary>
        {otherPlatforms.map(([id, steps]) => (
          <div key={id} className="mt-3">
            <h4 className="text-sm font-medium">{steps.title}</h4>
            <ol className="mt-1 list-decimal space-y-1 pl-5 text-sm">
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
    </section>
  );
}
