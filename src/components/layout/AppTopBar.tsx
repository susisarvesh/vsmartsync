import { DatabaseStatusControl } from "@/components/DatabaseStatusControl";
import type { DatabaseStatus } from "@/types/database";

type AppTopBarProps = {
  title: string;
  database: DatabaseStatus | null;
  databaseLoading?: boolean;
  databaseError?: string | null;
  retrying?: boolean;
  onRetryDatabase?: () => void;
};

export function AppTopBar({
  title,
  database,
  databaseLoading = false,
  databaseError = null,
  retrying = false,
  onRetryDatabase,
}: AppTopBarProps) {
  return (
    <header className="flex h-16 shrink-0 items-center justify-between gap-3 border-b border-slate-300 bg-white px-4 md:px-6">
      <h1 className="min-w-0 flex-1 truncate text-base font-semibold leading-none text-slate-900 sm:text-lg">
        {title}
      </h1>
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
