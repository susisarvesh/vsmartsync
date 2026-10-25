import { useEffect, useRef, useState, type ReactNode } from "react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppTopBar } from "@/components/layout/AppTopBar";
import type { DatabaseStatus } from "@/types/database";
import { pageTitle, type AppRoute } from "@/navigation";
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
  const [labelsVisible, setLabelsVisible] = useState(false);
  const openTimer = useRef<number | null>(null);
  const closeTimer = useRef<number | null>(null);
  const labelTimer = useRef<number | null>(null);

  useEffect(() => {
    document.title = `${pageTitle(route)} | Vsmart Sync`;
  }, [route]);

  useEffect(() => {
    return () => {
      if (openTimer.current != null) {
        window.clearTimeout(openTimer.current);
      }
      if (closeTimer.current != null) {
        window.clearTimeout(closeTimer.current);
      }
      if (labelTimer.current != null) {
        window.clearTimeout(labelTimer.current);
      }
    };
  }, []);

  function clearTimers() {
    if (openTimer.current != null) {
      window.clearTimeout(openTimer.current);
      openTimer.current = null;
    }
    if (closeTimer.current != null) {
      window.clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
    if (labelTimer.current != null) {
      window.clearTimeout(labelTimer.current);
      labelTimer.current = null;
    }
  }

  function openNavigation() {
    clearTimers();
    openTimer.current = window.setTimeout(() => {
      setLabelsVisible(true);
      setNavigationOpen(true);
    }, 80);
  }

  function closeNavigation() {
    clearTimers();
    closeTimer.current = window.setTimeout(() => {
      setNavigationOpen(false);
      labelTimer.current = window.setTimeout(() => setLabelsVisible(false), 300);
    }, 160);
  }

  return (
    <div className="flex h-full min-h-0 bg-background text-foreground">
      <div
        id="primary-navigation"
        onMouseEnter={openNavigation}
        onMouseLeave={closeNavigation}
        className={cn(
          "h-full shrink-0 overflow-hidden border-r border-white/10 bg-linear-to-b from-[#3355e0] via-[#1f3aa8] to-[#0d1440] shadow-[4px_0_24px_rgba(0,0,0,0.25)] transition-[width] duration-300 ease-in-out motion-reduce:transition-none",
          navigationOpen ? "w-64" : "w-16",
        )}
      >
        <AppSidebar route={route} expanded={labelsVisible} onNavigate={onNavigate} />
      </div>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <AppTopBar
          title={pageTitle(route)}
          database={database}
          databaseLoading={databaseLoading}
          databaseError={databaseError}
          retrying={retrying}
          onRetryDatabase={onRetryDatabase}
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
