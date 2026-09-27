export type UserStatus = "active" | "inactive";

export type UserOnDevice = {
  id: string;
  userId: string;
  deviceId: string;
  username: string;
  status: UserStatus;
  createdAt: string;
  provisionedAt: string | null;
};

export type UserDeviceAssignment = {
  id: string;
  userId: string;
  deviceId: string;
  deviceName: string;
  host: string;
  port: number;
  createdAt: string;
};

export type User = {
  id: string;
  username: string;
  status: UserStatus;
  createdAt: string;
  updatedAt: string;
};
