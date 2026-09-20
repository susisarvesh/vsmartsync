import { useEffect, useState } from "react";
import { getAppInfo } from "../services/app";
import type { AppInfo } from "../types/app";

type AppInfoState = {
  info: AppInfo | null;
  error: string | null;
  loading: boolean;
};

export function useAppInfo(): AppInfoState {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    getAppInfo()
      .then((data) => {
        if (!cancelled) {
          setInfo(data);
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

    return () => {
      cancelled = true;
    };
  }, []);

  return { info, error, loading };
}
