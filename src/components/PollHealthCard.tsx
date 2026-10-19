import { useState } from "react";
import { Button } from "@/components/ui/button";
import { usePollStatus } from "@/hooks/useEvents";
import type { PollStatus } from "@/types/events";

function readerAction(lastError: string | null, needsOperator: boolean, retries: number): string {
  if (lastError === "SECRET_CORRUPT") {
    return "Set the device password again";
  }
  if (lastError === "SECRET_UNAVAILABLE") {
    return "The workstation key could not be read";
  }
  if (needsOperator) {
    return "Check the device login";
  }
  return `${retries} retries`;
}

function formatWhen(value: string | null): string {
  if (!value) {
    return "—";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

export function PollHealthCard() {
  const { data, error, isLoading } = usePollStatus();
  const [copyMessage, setCopyMessage] = useState<string | null>(null);
  const poolFull = Boolean(
    data && data.poolSize > 0 && data.poolInUse >= data.poolSize && data.poolIdle === 0,
  );

  async function copyDiagnostics(status: PollStatus) {
    const text = JSON.stringify(status, null, 2);
    try {
      await navigator.clipboard.writeText(text);
      setCopyMessage("Copied diagnostics");
    } catch {
      setCopyMessage("Select the diagnostics below and copy them");
    }
  }

  return (
    <section className="mb-5 rounded-lg border border-border bg-card p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-foreground">Reader polling</h3>
          <p className="mt-0.5 text-xs text-muted-foreground">
            In-memory status for this session. Passwords and event payloads are not included.
          </p>
        </div>
        {data ? (
          <Button type="button" variant="outline" size="sm" onClick={() => copyDiagnostics(data)}>
            Copy diagnostics
          </Button>
        ) : null}
      </div>
      {copyMessage ? (
        <p className="mt-2 text-xs text-muted-foreground">{copyMessage}</p>
      ) : null}
      {isLoading ? (
        <p className="mt-3 text-sm text-muted-foreground">Loading poll status…</p>
      ) : null}
      {error ? (
        <p className="mt-3 text-sm text-destructive" role="alert">
          {error instanceof Error ? error.message : String(error)}
        </p>
      ) : null}
      {data ? (
        <>
          <dl className="mt-3 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-4">
            <div>
              <dt className="text-xs text-muted-foreground">Active readers</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{data.configuredReaders}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Active requests</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{data.activeRequests}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Cooling down</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{data.coolingDown}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Failing</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{data.failing}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Concurrency</dt>
              <dd className="mt-0.5 font-medium tabular-nums">
                {data.concurrency} / {data.maxConcurrency}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Last cycle</dt>
              <dd className="mt-0.5 font-medium tabular-nums">
                {data.lastCycleMs == null ? "—" : `${data.lastCycleMs} ms`}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Reached / failed</dt>
              <dd className="mt-0.5 font-medium tabular-nums">
                {data.lastCycleReached} / {data.lastCycleFailed}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Visit time</dt>
              <dd className="mt-0.5 font-medium tabular-nums">
                {data.visitP50Ms == null || data.visitP95Ms == null
                  ? "—"
                  : `p50 ${data.visitP50Ms} ms, p95 ${data.visitP95Ms} ms`}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Last checkpoint</dt>
              <dd className="mt-0.5 font-medium tabular-nums">
                {data.lastCheckpointAt == null
                  ? "—"
                  : `${data.lastCheckpointReader ?? "Reader"} ${data.lastCheckpointRollOver ?? 0}:${data.lastCheckpointSeq ?? 0}`}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Events stored this session</dt>
              <dd className="mt-0.5 font-medium tabular-nums">{data.eventsPersisted}</dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Database pool</dt>
              <dd className="mt-0.5 font-medium tabular-nums">
                {data.poolInUse} in use, {data.poolIdle} idle, max {data.poolSize}
              </dd>
            </div>
          </dl>
          <p className="mt-3 text-xs text-muted-foreground">{data.guidance}</p>
          {data.failures.secretCorrupt > 0 ? (
            <p className="mt-2 text-xs text-destructive" role="status">
              {data.failures.secretCorrupt} stored passwords could not be decrypted. Set those
              device passwords again.
            </p>
          ) : null}
          {poolFull ? (
            <p className="mt-2 text-xs text-destructive" role="status">
              The database pool is full. Set VSMART_POLL_CONCURRENCY={data.recommendedConcurrency}{" "}
              if enrollment or the window starts waiting.
            </p>
          ) : null}
          {data.devices.length === 0 ? (
            <p className="mt-3 text-sm text-muted-foreground">No readers are waiting to retry.</p>
          ) : (
            <div className="mt-3 overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead className="text-muted-foreground">
                  <tr>
                    <th className="py-1 pr-3 font-medium">Reader</th>
                    <th className="py-1 pr-3 font-medium">Last success</th>
                    <th className="py-1 pr-3 font-medium">Last error</th>
                    <th className="py-1 pr-3 font-medium">Next retry</th>
                    <th className="py-1 font-medium">Action</th>
                  </tr>
                </thead>
                <tbody>
                  {data.devices.map((device) => (
                    <tr key={device.deviceId} className="border-t border-border">
                      <td className="py-1.5 pr-3 font-medium text-foreground">{device.deviceName}</td>
                      <td className="py-1.5 pr-3">{formatWhen(device.lastSuccess)}</td>
                      <td className="py-1.5 pr-3 tabular-nums">{device.lastError ?? "—"}</td>
                      <td className="py-1.5 pr-3">{formatWhen(device.nextRetryAt)}</td>
                      <td className="py-1.5">
                        {readerAction(
                          device.lastError,
                          device.needsOperator,
                          device.consecutiveFailures,
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      ) : null}
    </section>
  );
}
