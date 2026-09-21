import type { ReactNode } from "react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppTopBar } from "@/components/layout/AppTopBar";
import type { DatabaseStatus } from "@/types/database";
import type { AppRoute } from "@/navigation";

type AppShellProps = {
  route: AppRoute;
  onNavigate: (route: AppRoute) => void;
  database: DatabaseStatus | null;
  databaseLoading?: boolean;
  databaseError?: string | null;
  retrying?: boolean;
  onRetryDatabase?: () => void;
  children: ReactNode;
};

export function AppShell({
  route,
  onNavigate,
  database,
  databaseLoading,
  databaseError,
  retrying,
  onRetryDatabase,
  children,
}: AppShellProps) {
  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <div className="flex min-h-0 flex-1">
        <AppSidebar route={route} onNavigate={onNavigate} />
        <div className="flex min-w-0 flex-1 flex-col">
          <AppTopBar
            database={database}
            databaseLoading={databaseLoading}
            databaseError={databaseError}
            retrying={retrying}
            onRetryDatabase={onRetryDatabase}
          />
          <main className="min-w-0 flex-1 overflow-auto">
            <div className="mx-auto w-full max-w-6xl p-5 md:p-6">{children}</div>
          </main>
        </div>
      </div>
    </div>
  );
}
