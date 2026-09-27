export type EnrollmentStatus =
  | "pending"
  | "active"
  | "failed"
  | "cancelled"
  | "revoked";

export type Enrollment = {
  id: string;
  userId: string;
  userName: string;
  credentialId: string;
  credentialType: string;
  maskedValue: string | null;
  deviceId: string;
  deviceName: string;
  status: EnrollmentStatus;
  activatedAt: string | null;
  cancelledAt: string | null;
  revokedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CreateEnrollmentInput = {
  userId: string;
  credentialId: string;
  deviceId: string;
};

export type EnrollmentOption = {
  enrollType: string;
  label: string;
};

export type DeviceEnrollmentOptions = {
  deviceId: string;
  options: EnrollmentOption[];
};

export type ListEnrollmentsFilter = {
  userId?: string;
  deviceId?: string;
  credentialId?: string;
  status?: EnrollmentStatus;
  limit?: number;
  offset?: number;
};
