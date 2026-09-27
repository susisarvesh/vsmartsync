export type ConnectionStatus = "unknown" | "online" | "offline";

export type DeviceStatus = "active" | "inactive";

export type Device = {
  id: string;
  deviceName: string;
  host: string;
  port: number;
  macAddress: string | null;
  deviceModel: string | null;
  username: string;
  status: DeviceStatus;
  connectionStatus: ConnectionStatus;
  lastSeenAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CreateDeviceInput = {
  deviceName: string;
  host: string;
  port: number;
  macAddress: string;
  username: string;
  password: string;
};

export type UpdateDeviceInput = {
  id: string;
  deviceName: string;
  host: string;
  port: number;
  macAddress: string;
  username: string;
};
