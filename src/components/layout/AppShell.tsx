import type { ReactNode } from "react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppTopBar } from "@/components/layout/AppTopBar";
import type { DatabaseStatus } from "@/types/database";
import type { AppRoute } from "@/navigation";

type AppShellProps = {
  route: AppRoute;
  onNavigate: (route: AppRoute) => void;
  database: DatabaseStatus | null;
  children: ReactNode;
};

export function AppShell({
  route,
  onNavigate,
  database,
  children,
}: AppShellProps) {
  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <AppTopBar database={database} />
      <div className="flex min-h-0 flex-1">
        <AppSidebar route={route} onNavigate={onNavigate} />
        <main className="min-w-0 flex-1 overflow-auto p-4 md:p-5">
          {children}
        </main>
      </div>
    </div>
  );
}
