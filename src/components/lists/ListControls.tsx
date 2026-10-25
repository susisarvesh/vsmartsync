import { Search } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function SearchField({
  value,
  onChange,
  placeholder,
  label,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  label: string;
}) {
  return (
    <div className="relative min-w-[14rem] flex-1">
      <Search
        className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
        aria-hidden
      />
      <Input
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="pl-8"
        aria-label={label}
      />
    </div>
  );
}

export function ResultSummary({
  start,
  count,
  total,
  noun,
}: {
  start: number;
  count: number;
  total: number;
  noun: string;
}) {
  const label =
    total === 0
      ? `No ${noun}`
      : `Showing ${(start + 1).toLocaleString()}–${(start + count).toLocaleString()} of ${total.toLocaleString()} ${noun}`;
  return <p className="text-xs text-muted-foreground">{label}</p>;
}

export function PageControls({
  page,
  pageCount,
  onPage,
}: {
  page: number;
  pageCount: number;
  onPage: (page: number) => void;
}) {
  if (pageCount <= 1) {
    return null;
  }
  return (
    <div className="flex items-center justify-end gap-2">
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={page <= 1}
        onClick={() => onPage(page - 1)}
      >
        Previous
      </Button>
      <span className="text-xs tabular-nums text-muted-foreground">
        Page {page.toLocaleString()} of {pageCount.toLocaleString()}
      </span>
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={page >= pageCount}
        onClick={() => onPage(page + 1)}
      >
        Next
      </Button>
    </div>
  );
}

export function FilterBar({ children }: { children: ReactNode }) {
  return <div className="mb-3 flex flex-wrap items-center gap-2">{children}</div>;
}
