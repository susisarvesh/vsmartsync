import { invoke } from "@tauri-apps/api/core";
import type { AccessEvent, FetchDeviceEventsResult, PollStatus } from "@/types/events";

export async function listAccessEvents(deviceId?: string | null): Promise<AccessEvent[]> {
  return invoke<AccessEvent[]>("list_access_events", {
    deviceId: deviceId ?? null,
    limit: null,
  });
}

export async function fetchDeviceEvents(deviceId: string): Promise<FetchDeviceEventsResult> {
  return invoke<FetchDeviceEventsResult>("fetch_device_events", { deviceId });
}

export async function getPollStatus(): Promise<PollStatus> {
  return invoke<PollStatus>("get_poll_status");
}
