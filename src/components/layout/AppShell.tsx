import { useEffect, useState, type ReactNode } from "react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppTopBar } from "@/components/layout/AppTopBar";
import type { DatabaseStatus } from "@/types/database";
import type { AppRoute } from "@/navigation";
import { cn } from "@/lib/utils";

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
  const [navigationOpen, setNavigationOpen] = useState(false);

  useEffect(() => {
    if (!navigationOpen) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setNavigationOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [navigationOpen]);

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <div
        id="primary-navigation"
        onMouseEnter={() => setNavigationOpen(true)}
        onMouseLeave={() => setNavigationOpen(false)}
        className={cn(
          "h-full shrink-0 overflow-hidden border-r border-border bg-card transition-[width] duration-200 ease-out motion-reduce:transition-none",
          navigationOpen ? "w-60" : "w-16",
        )}
      >
        <AppSidebar
          route={route}
          expanded={navigationOpen}
          onNavigate={onNavigate}
        />
      </div>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <AppTopBar
          database={database}
          databaseLoading={databaseLoading}
          databaseError={databaseError}
          retrying={retrying}
          onRetryDatabase={onRetryDatabase}
          onGoToDashboard={() => onNavigate("dashboard")}
          navigationOpen={navigationOpen}
          onToggleNavigation={() => setNavigationOpen((open) => !open)}
        />
        <main className="min-h-0 flex-1 overflow-hidden">
          <div className="mx-auto flex h-full w-full max-w-6xl flex-col overflow-auto p-4 md:p-5">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
