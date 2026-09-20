export type ConnectionStatus = "unknown" | "online" | "offline";

export type Device = {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  connectionStatus: ConnectionStatus;
  lastSeenAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CreateDeviceInput = {
  name: string;
  host: string;
  port: number;
  username: string;
  password: string;
};

export type UpdateDeviceInput = {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
};
