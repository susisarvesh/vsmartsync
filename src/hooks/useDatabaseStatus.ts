import { useCallback, useEffect, useState } from "react";
import { connectDatabase, getDatabaseStatus } from "@/services/database";
import type { DatabaseStatus } from "@/types/database";

export function useDatabaseStatus() {
  const [status, setStatus] = useState<DatabaseStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [retrying, setRetrying] = useState(false);

  const refresh = useCallback(async (forceConnect: boolean) => {
    try {
      const next = forceConnect
        ? await connectDatabase()
        : await getDatabaseStatus();
      setStatus(next);
      setError(null);
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
      setRetrying(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    function apply(next: DatabaseStatus) {
      if (!cancelled) {
        setStatus(next);
        setError(null);
        setLoading(false);
      }
    }

    getDatabaseStatus()
      .then(apply)
      .catch((reason: unknown) => {
        if (!cancelled) {
          setError(reason instanceof Error ? reason.message : String(reason));
          setLoading(false);
        }
      });

    const delay = status?.connected ? 30_000 : 5_000;
    const timer = window.setInterval(() => {
      void getDatabaseStatus()
        .then(apply)
        .catch(() => {
          /* keep last known status while polling */
        });
    }, delay);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [status?.connected]);

  const retry = useCallback(() => {
    setRetrying(true);
    void refresh(true);
  }, [refresh]);

  return { status, error, loading, retrying, retry };
}
