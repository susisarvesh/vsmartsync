import { useState } from "react";
import { Circle, Database } from "lucide-react";
import { PostgresSetupDetails } from "@/components/PostgresSetupCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={cn(
          "inline-flex items-center gap-2 rounded-md border border-border bg-card text-left transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          size === "sm" ? "px-2 py-1" : "px-3 py-2",
        )}
        aria-label={`PostgreSQL ${label}. Open connection details.`}
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

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Database className="h-4 w-4 text-muted-foreground" aria-hidden />
              PostgreSQL
            </DialogTitle>
            <DialogDescription>
              Local database connection for this workstation.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3">
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

          <DialogFooter>
            {onRetry && !connected ? (
              <Button
                type="button"
                onClick={onRetry}
                disabled={retrying}
              >
                {retrying ? "Connecting…" : "Try again"}
              </Button>
            ) : null}
            <Button
              type="button"
              variant="secondary"
              onClick={() => setOpen(false)}
            >
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
