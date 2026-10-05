import { useEffect, useRef, useState, type ReactNode } from "react";
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
  const [navigationHovered, setNavigationHovered] = useState(false);
  const [navigationPinned, setNavigationPinned] = useState(false);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const navigationOpen = navigationHovered || navigationPinned;

  const clearCloseTimer = () => {
    if (closeTimer.current !== null) {
      clearTimeout(closeTimer.current);
      closeTimer.current = null;
    }
  };

  const scheduleHoverClose = () => {
    clearCloseTimer();
    closeTimer.current = setTimeout(() => {
      setNavigationHovered(false);
      closeTimer.current = null;
    }, 220);
  };

  const closeNavigation = () => {
    clearCloseTimer();
    setNavigationHovered(false);
    setNavigationPinned(false);
  };

  useEffect(() => {
    if (!navigationOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeNavigation();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [navigationOpen]);

  useEffect(() => () => clearCloseTimer(), []);

  return (
    <div className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <div className="relative z-50">
        <AppTopBar
          database={database}
          databaseLoading={databaseLoading}
          databaseError={databaseError}
          retrying={retrying}
          onRetryDatabase={onRetryDatabase}
          onGoToDashboard={() => {
            onNavigate("dashboard");
            closeNavigation();
          }}
          navigationOpen={navigationOpen}
          onToggleNavigation={() => {
            clearCloseTimer();
            setNavigationHovered(false);
            setNavigationPinned((pinned) => !pinned);
          }}
          onNavigationHoverStart={() => {
            clearCloseTimer();
            setNavigationHovered(true);
          }}
          onNavigationHoverEnd={scheduleHoverClose}
        />
      </div>
      <div className="flex min-h-0 flex-1">
        <main className="min-w-0 flex-1 overflow-auto">
          <div className="mx-auto w-full max-w-6xl p-4 md:p-5">{children}</div>
        </main>
      </div>
      <button
        type="button"
        aria-label="Close navigation"
        tabIndex={navigationOpen ? 0 : -1}
        onClick={closeNavigation}
        className={cn(
          "fixed inset-x-0 bottom-0 top-11 z-40 bg-black/20 transition-opacity duration-200 motion-reduce:transition-none",
          navigationOpen
            ? "opacity-100"
            : "pointer-events-none opacity-0",
        )}
      />
      <div
        id="primary-navigation"
        aria-hidden={!navigationOpen}
        onMouseEnter={() => {
          clearCloseTimer();
          setNavigationHovered(true);
        }}
        onMouseLeave={scheduleHoverClose}
        className={cn(
          "fixed bottom-0 left-0 top-11 z-50 w-60 shadow-lg transition-[transform,visibility] duration-200 ease-out motion-reduce:transition-none",
          navigationOpen
            ? "visible translate-x-0"
            : "invisible -translate-x-full",
        )}
      >
        <AppSidebar
          route={route}
          onNavigate={(nextRoute) => {
            onNavigate(nextRoute);
            closeNavigation();
          }}
        />
      </div>
    </div>
  );
}
