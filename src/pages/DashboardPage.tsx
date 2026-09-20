import { Users } from "lucide-react";
import { DatabaseStatusControl } from "@/components/DatabaseStatusControl";
import { PageHeader } from "@/components/layout/PageHeader";
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
          <div className="flex items-center justify-between gap-3">
            <div>
              <h3 className="text-sm font-semibold">PostgreSQL</h3>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Click status for details
              </p>
            </div>
            <DatabaseStatusControl
              database={database}
              loading={databaseLoading}
              error={databaseError}
              retrying={retrying}
              onRetry={onRetryDatabase}
              size="md"
            />
          </div>
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
    </div>
  );
}
