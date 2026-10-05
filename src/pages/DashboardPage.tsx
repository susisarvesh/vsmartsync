import {
  ChevronRight,
  CreditCard,
  HardDrive,
  UserPlus,
  Users,
} from "lucide-react";
import { PageHeader } from "@/components/layout/PageHeader";
import { useAppInfo } from "@/hooks/useAppInfo";
import { cn } from "@/lib/utils";
import type { AppRoute } from "@/navigation";
import type { DatabaseStatus } from "@/types/database";

type DashboardPageProps = {
  database: DatabaseStatus | null;
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
    id: "assign-device",
    label: "Assign to device",
    description: "Choose a device, then assign users to it",
    icon: UserPlus,
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
    description: "User credentials, enrolled on a device",
    icon: CreditCard,
  },
];

export function DashboardPage({
  database,
  onNavigate,
}: DashboardPageProps) {
  const { info, loading: infoLoading, error: infoError } = useAppInfo();
  const connected = Boolean(database?.connected);

  return (
    <div>
      <PageHeader
        title="Workstation Dashboard"
        description="Operational status for this workstation. Only installed modules are shown."
        titleClassName="text-2xl font-bold tracking-normal"
      />

      <section className="mb-5">
        <div className="mb-2 flex items-end justify-between gap-2">
          <h3 className="text-sm font-semibold text-foreground">Modules</h3>
          {!connected ? (
            <p className="text-xs text-muted-foreground">
              Connect PostgreSQL to use modules
            </p>
          ) : null}
        </div>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {MODULES.map((module) => {
            const Icon = module.icon;
            return (
              <button
                key={module.id}
                type="button"
                onClick={() => onNavigate(module.id)}
                disabled={!connected}
                className={cn(
                  "group flex min-h-28 items-center gap-3 rounded-md border border-border bg-card p-4 text-left shadow-sm transition-colors",
                  connected
                    ? "hover:border-primary/25 hover:bg-accent/30 hover:shadow-md"
                    : "cursor-not-allowed opacity-60",
                )}
              >
                <span className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md border border-primary/10 bg-accent text-primary">
                  <Icon className="h-5 w-5" aria-hidden />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-semibold text-foreground">
                    {module.label}
                  </span>
                  <span className="mt-0.5 block text-xs text-muted-foreground">
                    {module.description}
                  </span>
                </span>
                <ChevronRight
                  className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-primary"
                  aria-hidden
                />
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
