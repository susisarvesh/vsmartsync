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
    <header className="flex h-11 shrink-0 items-center justify-between gap-3 border-b border-border bg-card px-4">
      <p className="truncate text-xs text-muted-foreground">
        Workstation · local PostgreSQL · Matrix COSEC
      </p>
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
