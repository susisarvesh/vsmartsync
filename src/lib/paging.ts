import { useEffect, useState } from "react";

export function usePagedItems<T>(items: T[], pageSize: number, resetKey: string) {
  const [page, setPage] = useState(1);

  useEffect(() => {
    setPage(1);
  }, [resetKey]);

  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));
  const current = Math.min(page, pageCount);
  const start = items.length === 0 ? 0 : (current - 1) * pageSize;

  return {
    page: current,
    pageCount,
    start,
    total: items.length,
    slice: items.slice(start, start + pageSize),
    setPage,
  };
}
