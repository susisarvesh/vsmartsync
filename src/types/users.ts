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
  matrixUserId: string | null;
  shortName: string | null;
  fullName: string | null;
  referenceId: number | null;
  createdAt: string;
  updatedAt: string;
};

export type ImportRowError = {
  row: number;
  message: string;
};

export type ImportUsersResult = {
  imported: number;
  errors: ImportRowError[];
};
