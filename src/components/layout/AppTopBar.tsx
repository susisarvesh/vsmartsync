import { DatabaseStatusControl } from "@/components/DatabaseStatusControl";
import type { DatabaseStatus } from "@/types/database";

type AppTopBarProps = {
  database: DatabaseStatus | null;
  databaseLoading?: boolean;
  databaseError?: string | null;
  retrying?: boolean;
  onRetryDatabase?: () => void;
};

export function AppTopBar({
  database,
  databaseLoading = false,
  databaseError = null,
  retrying = false,
  onRetryDatabase,
}: AppTopBarProps) {
  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border bg-card px-4">
      <div className="min-w-0">
        <h1 className="truncate text-sm font-semibold text-foreground">
          Vsmart Sync
        </h1>
        <p className="truncate text-xs text-muted-foreground">
          Local access-control workstation
        </p>
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
