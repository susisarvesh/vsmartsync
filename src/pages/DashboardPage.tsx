import { Database, Users } from "lucide-react";
import { PageHeader } from "@/components/layout/PageHeader";
import { PostgresSetupCard } from "@/components/PostgresSetupCard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useAppInfo } from "@/hooks/useAppInfo";
import type { DatabaseStatus } from "@/types/database";

type DashboardPageProps = {
  database: DatabaseStatus | null;
  databaseLoading: boolean;
  databaseError: string | null;
  retrying: boolean;
  onRetryDatabase: () => void;
  onOpenUsers: () => void;
};

export function DashboardPage({
  database,
  databaseLoading,
  databaseError,
  retrying,
  onRetryDatabase,
  onOpenUsers,
}: DashboardPageProps) {
  const { info, loading: infoLoading, error: infoError } = useAppInfo();
  const connected = Boolean(database?.connected);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Operational status for this workstation. Only installed modules are shown."
        action={
          connected ? (
            <Button type="button" onClick={onOpenUsers}>
              <Users className="h-4 w-4" aria-hidden />
              Open Users
            </Button>
          ) : null
        }
      />

      <div className="grid gap-3 md:grid-cols-2">
        <section className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2">
            <Database className="h-4 w-4 text-muted-foreground" aria-hidden />
            <h3 className="text-sm font-semibold">PostgreSQL</h3>
          </div>
          {databaseLoading && !database ? (
            <p className="mt-3 text-sm text-muted-foreground">
              Checking local database…
            </p>
          ) : null}
          {databaseError ? (
            <p className="mt-3 text-sm text-destructive" role="alert">
              {databaseError}
            </p>
          ) : null}
          {database ? (
            <div className="mt-3 space-y-2">
              <Badge variant={connected ? "success" : "warning"}>
                <span aria-hidden>●</span>
                {connected ? "Connected" : "Unavailable"}
              </Badge>
              <p className="text-sm text-muted-foreground">{database.message}</p>
              {!connected ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={onRetryDatabase}
                  disabled={retrying}
                >
                  {retrying ? "Connecting…" : "Try again"}
                </Button>
              ) : null}
            </div>
          ) : null}
        </section>

        <section className="rounded-lg border border-border bg-card p-4">
          <h3 className="text-sm font-semibold">Application</h3>
          {infoLoading ? (
            <p className="mt-3 text-sm text-muted-foreground">
              Loading application info…
            </p>
          ) : null}
          {infoError ? (
            <p className="mt-3 text-sm text-destructive" role="alert">
              {infoError}
            </p>
          ) : null}
          {info ? (
            <dl className="mt-3 grid grid-cols-[7rem_1fr] gap-x-3 gap-y-2 text-sm">
              <dt className="text-muted-foreground">Name</dt>
              <dd>{info.name}</dd>
              <dt className="text-muted-foreground">Version</dt>
              <dd>{info.version}</dd>
              <dt className="text-muted-foreground">Stage</dt>
              <dd>{info.stage}</dd>
            </dl>
          ) : null}
        </section>
      </div>

      {database && !database.connected ? (
        <div className="mt-4">
          <PostgresSetupCard
            status={database}
            retrying={retrying}
            onRetry={onRetryDatabase}
          />
        </div>
      ) : null}
    </div>
  );
}
