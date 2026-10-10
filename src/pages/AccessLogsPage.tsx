import { useState } from "react";
import { EmptyState, ErrorState, PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useToast } from "@/components/ui/toaster";
import { useDevicesQuery } from "@/hooks/useDevices";
import { useAccessEvents, useFetchDeviceEvents } from "@/hooks/useEvents";

type AccessLogsPageProps = {
  enabled: boolean;
};

export function AccessLogsPage({ enabled }: AccessLogsPageProps) {
  const { toast } = useToast();
  const devicesQuery = useDevicesQuery(enabled);
  const [deviceId, setDeviceId] = useState("");
  const eventsQuery = useAccessEvents(enabled, deviceId || null);
  const fetchEvents = useFetchDeviceEvents();
  const activeDevices = (devicesQuery.data ?? []).filter((device) => device.status === "active");
  const rows = eventsQuery.data ?? [];

  async function onFetch() {
    if (!deviceId) {
      return;
    }
    try {
      const result = await fetchEvents.mutateAsync(deviceId);
      if (result.anchored) {
        toast({
          title: "Logging started",
          description:
            "Events from this moment on will be saved. Earlier history on the device is not imported.",
        });
        return;
      }
      toast({
        title: result.imported > 0 ? "Events saved" : "No new events",
        description:
          result.imported > 0
            ? `${result.imported} event${result.imported === 1 ? "" : "s"} saved.`
            : "The device has no newer card or face events.",
      });
    } catch (reason: unknown) {
      toast({
        title: "Could not fetch events",
        description: message(reason),
        variant: "destructive",
      });
    }
  }

  return (
    <div>
      <PageHeader
        title="In/Out Report"
        description="Entries the device already allowed or recorded after someone presented a card or face. The time, name, and device are saved here."
        action={
          <Button
            type="button"
            disabled={!deviceId || fetchEvents.isPending}
            onClick={() => void onFetch()}
          >
            {fetchEvents.isPending ? "Fetching…" : "Fetch from device"}
          </Button>
        }
      />
      <div className="mb-4 grid max-w-md gap-1.5">
        <Label>Device</Label>
        <Select
          value={deviceId || "all"}
          onValueChange={(value) => setDeviceId(value === "all" ? "" : value)}
        >
          <SelectTrigger aria-label="Access log device">
            <SelectValue placeholder="All devices" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All devices</SelectItem>
            {activeDevices.map((device) => (
              <SelectItem key={device.id} value={device.id}>
                {device.deviceName}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      {eventsQuery.isLoading ? (
        <p className="text-sm text-muted-foreground">Loading access logs…</p>
      ) : null}
      {eventsQuery.isError ? (
        <ErrorState
          title="Could not load access logs"
          description={message(eventsQuery.error)}
          onRetry={() => void eventsQuery.refetch()}
        />
      ) : null}
      {!eventsQuery.isLoading && !eventsQuery.isError && rows.length === 0 ? (
        <EmptyState
          title="No access logs yet"
          description="Logging starts from the device's current event. Choose a device, fetch it, then present a card or face. Earlier history is not imported."
        />
      ) : null}
      {rows.length > 0 ? (
        <div className="overflow-x-auto rounded-lg border border-border bg-card">
          <table className="w-full min-w-[40rem] border-collapse text-sm">
            <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="whitespace-nowrap px-3 py-2 font-medium">Time</th>
                <th className="whitespace-nowrap px-3 py-2 font-medium">Name</th>
                <th className="whitespace-nowrap px-3 py-2 font-medium">Device</th>
                <th className="whitespace-nowrap px-3 py-2 font-medium">Event</th>
                <th className="whitespace-nowrap px-3 py-2 font-medium">Details</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((event) => (
                <tr key={event.id} className="border-t border-border">
                  <td className="whitespace-nowrap px-3 py-2 text-foreground">
                    {event.occurredLabel}
                  </td>
                  <td className="px-3 py-2 font-medium text-foreground">
                    {event.personName ?? "Unknown"}
                  </td>
                  <td className="px-3 py-2 text-foreground">{event.deviceName}</td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {event.eventId ?? "Not reported"}
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {event.details || "Not reported"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </div>
  );
}

function message(reason: unknown): string {
  if (typeof reason === "string") {
    return reason;
  }
  if (reason instanceof Error) {
    return reason.message;
  }
  return "The access log could not be loaded.";
}
