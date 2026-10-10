import { Menu } from "lucide-react";
import { DatabaseStatusControl } from "@/components/DatabaseStatusControl";
import { Button } from "@/components/ui/button";
import type { DatabaseStatus } from "@/types/database";

type AppTopBarProps = {
  database: DatabaseStatus | null;
  databaseLoading?: boolean;
  databaseError?: string | null;
  retrying?: boolean;
  onRetryDatabase?: () => void;
  onGoToDashboard: () => void;
  navigationOpen: boolean;
  onToggleNavigation: () => void;
  onNavigationHoverStart: () => void;
  onNavigationHoverEnd: () => void;
};

export function AppTopBar({
  database,
  databaseLoading = false,
  databaseError = null,
  retrying = false,
  onRetryDatabase,
  onGoToDashboard,
  navigationOpen,
  onToggleNavigation,
  onNavigationHoverStart,
  onNavigationHoverEnd,
}: AppTopBarProps) {
  return (
    <header className="flex h-11 shrink-0 items-center justify-between gap-3 border-b border-border bg-card px-4">
      <div className="flex min-w-0 items-center gap-2">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={onToggleNavigation}
          onMouseEnter={onNavigationHoverStart}
          onMouseLeave={onNavigationHoverEnd}
          aria-label={navigationOpen ? "Close navigation menu" : "Open navigation menu"}
          aria-expanded={navigationOpen}
          aria-controls="primary-navigation"
          title={navigationOpen ? "Close navigation menu" : "Open navigation menu"}
        >
          <Menu className="h-4 w-4" aria-hidden />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="px-1.5 font-semibold"
          onClick={onGoToDashboard}
          aria-label="Go to Dashboard"
        >
          Vsmart Sync
        </Button>
        <span className="hidden truncate text-xs text-muted-foreground sm:inline">
          Local access control workstation
        </span>
      </div>
      <DatabaseStatusControl
        database={database}
        loading={databaseLoading}
        error={databaseError}
        retrying={retrying}
        onRetry={onRetryDatabase}
        size="sm"
      />
    </header>
  );
}
