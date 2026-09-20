import { Badge } from "@/components/ui/badge";
import type { DatabaseStatus } from "@/types/database";

type AppTopBarProps = {
  database: DatabaseStatus | null;
};

export function AppTopBar({ database }: AppTopBarProps) {
  const connected = Boolean(database?.connected);

  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-card px-4">
      <div className="min-w-0">
        <h1 className="truncate text-sm font-semibold text-foreground">
          Vsmart Sync
        </h1>
        <p className="truncate text-xs text-muted-foreground">
          Local access-control workstation
        </p>
      </div>
      <div className="flex items-center gap-2">
        {database ? (
          <Badge variant={connected ? "success" : "warning"}>
            <span aria-hidden>●</span>
            <span>
              {connected
                ? `PostgreSQL · ${database.host}:${database.port}`
                : "PostgreSQL unavailable"}
            </span>
          </Badge>
        ) : (
          <Badge>Checking database…</Badge>
        )}
      </div>
    </header>
  );
}
