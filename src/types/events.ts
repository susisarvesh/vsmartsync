export type AccessEvent = {
  id: string;
  deviceId: string;
  deviceName: string;
  occurredLabel: string;
  deviceDate: string;
  deviceTime: string;
  personName: string | null;
  eventId: string | null;
  details: string;
  ingestedAt: string;
};

export type FetchDeviceEventsResult = {
  deviceId: string;
  imported: number;
  anchored: boolean;
};
