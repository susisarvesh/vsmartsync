import {
  ChevronRight,
  CreditCard,
  HardDrive,
  Link2,
  Users,
} from "lucide-react";
import { DatabaseStatusControl } from "@/components/DatabaseStatusControl";
import { PageHeader } from "@/components/layout/PageHeader";
import { useAppInfo } from "@/hooks/useAppInfo";
import { cn } from "@/lib/utils";
import type { AppRoute } from "@/navigation";
import type { DatabaseStatus } from "@/types/database";

type DashboardPageProps = {
  database: DatabaseStatus | null;
  databaseLoading: boolean;
  databaseError: string | null;
  retrying: boolean;
  onRetryDatabase: () => void;
  onNavigate: (route: AppRoute) => void;
};

const MODULES: Array<{
  id: Exclude<AppRoute, "dashboard">;
  label: string;
  description: string;
  icon: typeof Users;
}> = [
  {
    id: "users",
    label: "Users",
    description: "People in the desired access state",
    icon: Users,
  },
  {
    id: "devices",
    label: "Devices",
    description: "Matrix COSEC doors and controllers",
    icon: HardDrive,
  },
  {
    id: "credentials",
    label: "Credentials",
    description: "Card and PIN records for users",
    icon: CreditCard,
  },
  {
    id: "enrollments",
    label: "Enrollments",
    description: "Desired credential-to-device assignments",
    icon: Link2,
  },
];

export function DashboardPage({
  database,
  databaseLoading,
  databaseError,
  retrying,
  onRetryDatabase,
  onNavigate,
}: DashboardPageProps) {
  const { info, loading: infoLoading, error: infoError } = useAppInfo();
  const connected = Boolean(database?.connected);

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Operational status for this workstation. Only installed modules are shown."
      />

      <section
        className={cn(
          "mb-5 rounded-lg border bg-card p-4",
          connected ? "border-border" : "border-red-200",
        )}
      >
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <h3 className="text-sm font-semibold text-foreground">
              Database
            </h3>
            <p className="mt-0.5 text-xs text-muted-foreground">
              Local PostgreSQL required for Users, Devices, Credentials, and
              Enrollments.
            </p>
            {database?.connected ? (
              <p className="mt-2 font-mono text-xs text-muted-foreground">
                {database.host}:{database.port} · {database.database}
              </p>
            ) : null}
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
        {databaseError && !connected ? (
          <p className="mt-3 text-sm text-destructive" role="alert">
            {databaseError}
          </p>
        ) : null}
      </section>

      <section className="mb-5">
        <div className="mb-2 flex items-end justify-between gap-2">
          <h3 className="text-sm font-semibold text-foreground">Modules</h3>
          {!connected ? (
            <p className="text-xs text-muted-foreground">
              Connect PostgreSQL to use modules
            </p>
          ) : null}
        </div>
        <div className="grid gap-3 sm:grid-cols-2">
          {MODULES.map((module) => {
            const Icon = module.icon;
            return (
              <button
                key={module.id}
                type="button"
                onClick={() => onNavigate(module.id)}
                disabled={!connected}
                className={cn(
                  "group flex items-start gap-3 rounded-lg border border-border bg-card p-4 text-left transition-colors",
                  connected
                    ? "hover:border-primary/30 hover:bg-accent/40"
                    : "cursor-not-allowed opacity-60",
                )}
              >
                <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-muted text-muted-foreground group-hover:border-primary/20 group-hover:text-primary">
                  <Icon className="h-4 w-4" aria-hidden />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="flex items-center justify-between gap-2">
                    <span className="text-sm font-semibold text-foreground">
                      {module.label}
                    </span>
                    <ChevronRight
                      className="h-4 w-4 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                      aria-hidden
                    />
                  </span>
                  <span className="mt-0.5 block text-xs text-muted-foreground">
                    {module.description}
                  </span>
                </span>
              </button>
            );
          })}
        </div>
      </section>

      <section className="rounded-lg border border-border bg-card p-4">
        <h3 className="text-sm font-semibold text-foreground">Application</h3>
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
          <dl className="mt-3 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-3">
            <div>
              <dt className="text-xs text-muted-foreground">Name</dt>
              <dd className="mt-0.5 font-medium">{info.name}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Version</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{info.version}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Stage</dt>
              <dd className="mt-0.5 font-medium">{info.stage}</dd>
            </div>
          </dl>
        ) : null}
      </section>
    </div>
  );
}
