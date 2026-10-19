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

export type DevicePollStatus = {
  deviceId: string;
  deviceName: string;
  lastSuccess: string | null;
  lastError: string | null;
  nextRetryAt: string | null;
  consecutiveFailures: number;
  needsOperator: boolean;
};

export type PollFailureCounts = {
  secretCorrupt: number;
  secretUnavailable: number;
  authFailed: number;
  timeout: number;
  unreachable: number;
  database: number;
  other: number;
};

export type PollStatus = {
  concurrency: number;
  recommendedConcurrency: number;
  maxConcurrency: number;
  intervalSecs: number;
  poolSize: number;
  foregroundReserve: number;
  guidance: string;
  activeRequests: number;
  configuredReaders: number;
  dueReaders: number;
  coolingDown: number;
  failing: number;
  eventsPersisted: number;
  lastCycleMs: number | null;
  lastCycleReached: number;
  lastCycleFailed: number;
  lastCycleEvents: number;
  visitP50Ms: number | null;
  visitP95Ms: number | null;
  queueP50Ms: number | null;
  decryptP50Ms: number | null;
  httpP50Ms: number | null;
  databaseP50Ms: number | null;
  maxInFlight: number;
  failures: PollFailureCounts;
  lastCheckpointReader: string | null;
  lastCheckpointRollOver: number | null;
  lastCheckpointSeq: number | null;
  lastCheckpointAt: string | null;
  poolInUse: number;
  poolIdle: number;
  devices: DevicePollStatus[];
};
