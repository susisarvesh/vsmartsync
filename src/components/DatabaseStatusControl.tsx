import { useEffect, useRef, useState } from "react";
import { Circle, Database, X } from "lucide-react";
import { PostgresSetupDetails } from "@/components/PostgresSetupCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { DatabaseStatus } from "@/types/database";
import { cn } from "@/lib/utils";

type DatabaseStatusControlProps = {
  database: DatabaseStatus | null;
  loading?: boolean;
  error?: string | null;
  retrying?: boolean;
  onRetry?: () => void;
  /** Compact for the top bar; slightly larger for the dashboard. */
  size?: "sm" | "md";
};

export function DatabaseStatusControl({
  database,
  loading = false,
  error = null,
  retrying = false,
  onRetry,
  size = "sm",
}: DatabaseStatusControlProps) {
  const [open, setOpen] = useState(false);
  const controlRef = useRef<HTMLDivElement>(null);
  const connected = Boolean(database?.connected);
  const checking = loading && !database;

  const label = checking
    ? "Checking"
    : connected
      ? "Connected"
      : error
        ? "Error"
        : "Failed";

  const variant = checking
    ? "default"
    : connected
      ? "success"
      : "destructive";

  const iconClass = checking
    ? "text-muted-foreground"
    : connected
      ? "text-success"
      : "text-destructive";

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: PointerEvent) => {
      if (event.target instanceof Node && !controlRef.current?.contains(event.target)) {
        setOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  return (
    <div ref={controlRef} className="relative shrink-0">
      <button
        type="button"
        onClick={() => setOpen((current) => !current)}
        className={cn(
          "inline-flex items-center gap-2 rounded-md border border-border bg-card text-left transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          size === "sm" ? "px-2 py-1" : "px-3 py-2",
        )}
        aria-label={`PostgreSQL ${label}. Open connection details.`}
        aria-controls="database-status-panel"
        aria-expanded={open}
      >
        <Database
          className={cn(
            "shrink-0 text-muted-foreground",
            size === "sm" ? "h-3.5 w-3.5" : "h-4 w-4",
          )}
          aria-hidden
        />
        <Badge variant={variant} className="pointer-events-none">
          <Circle
            className={cn("h-2.5 w-2.5 fill-current", iconClass)}
            aria-hidden
          />
          <span>{label}</span>
        </Badge>
      </button>

      {open ? (
        <section
          id="database-status-panel"
          role="dialog"
          aria-labelledby="database-status-title"
          className="absolute right-0 top-full z-50 mt-2 max-h-[calc(100vh-4rem)] w-96 max-w-[calc(100vw-2rem)] overflow-y-auto rounded-md border border-border bg-card text-left shadow-lg"
        >
          <header className="flex items-start justify-between gap-4 border-b border-border px-4 py-3">
            <div>
              <h2 id="database-status-title" className="flex items-center gap-2 text-sm font-semibold">
                <Database className="h-4 w-4 text-muted-foreground" aria-hidden />
                PostgreSQL
              </h2>
              <p className="mt-1 text-xs text-muted-foreground">
                Local database connection for this workstation.
              </p>
            </div>
            <button
              type="button"
              onClick={() => setOpen(false)}
              className="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
              aria-label="Close database status"
            >
              <X className="h-4 w-4" aria-hidden />
            </button>
          </header>

          <div className="space-y-3 p-4">
            <div className="flex items-center gap-2">
              <Badge variant={variant}>
                <Circle
                  className={cn("h-2.5 w-2.5 fill-current", iconClass)}
                  aria-hidden
                />
                <span>{label}</span>
              </Badge>
            </div>

            {error ? (
              <p className="text-sm text-destructive" role="alert">
                {error}
              </p>
            ) : null}

            {database ? (
              <dl className="grid grid-cols-[5.5rem_1fr] gap-x-3 gap-y-1.5 text-sm">
                <dt className="text-muted-foreground">Host</dt>
                <dd className="font-mono text-xs">
                  {database.host}:{database.port}
                </dd>
                <dt className="text-muted-foreground">Database</dt>
                <dd className="font-mono text-xs">{database.database}</dd>
                <dt className="text-muted-foreground">User</dt>
                <dd className="font-mono text-xs">{database.user}</dd>
                <dt className="text-muted-foreground">Status</dt>
                <dd className="text-sm">{database.message}</dd>
              </dl>
            ) : null}

            {checking ? (
              <p className="text-sm text-muted-foreground">
                Checking local database…
              </p>
            ) : null}

            {database && !database.connected ? (
              <PostgresSetupDetails status={database} />
            ) : null}
          </div>

          {onRetry && !connected ? (
            <div className="flex justify-end border-t border-border px-4 py-3">
              <Button
                type="button"
                onClick={onRetry}
                disabled={retrying}
              >
                {retrying ? "Connecting…" : "Try again"}
              </Button>
            </div>
          ) : null}
        </section>
      ) : null}
    </div>
  );
}
