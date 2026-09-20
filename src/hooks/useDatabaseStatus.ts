import { useCallback, useEffect, useState } from "react";
import { connectDatabase, getDatabaseStatus } from "../services/database";
import type { DatabaseStatus } from "../types/database";

type DatabaseStatusState = {
  status: DatabaseStatus | null;
  error: string | null;
  loading: boolean;
  retrying: boolean;
  retry: () => void;
};

export function useDatabaseStatus(): DatabaseStatusState {
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
    setLoading(true);
    getDatabaseStatus()
      .then((next) => {
        if (!cancelled) {
          setStatus(next);
          setError(null);
        }
      })
      .catch((reason: unknown) => {
        if (!cancelled) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    const timer = window.setInterval(() => {
      void getDatabaseStatus()
        .then((next) => {
          if (!cancelled) {
            setStatus(next);
            setError(null);
          }
        })
        .catch(() => {
          /* keep the last status while polling */
        });
    }, 5000);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const retry = useCallback(() => {
    setRetrying(true);
    void refresh(true);
  }, [refresh]);

  return { status, error, loading, retrying, retry };
}
