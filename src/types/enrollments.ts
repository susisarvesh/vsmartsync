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
  readerLabel: string | null;
  assignableCredentialTypes: string[];
  options: EnrollmentOption[];
};

export type EnrollmentSessionStatus =
  | "starting"
  | "waiting_for_card"
  | "processing"
  | "verifying"
  | "saving"
  | "success"
  | "failed"
  | "cancelled"
  | "timeout"
  | "busy"
  | "persistence_failed";

export type EnrollmentSession = {
  id: string;
  userId: string;
  userName: string;
  deviceId: string;
  deviceName: string;
  enrollType: string;
  status: EnrollmentSessionStatus;
  credentialId: string | null;
  errorCode: string | null;
  cardType: string | null;
  cardTypeLabel: string | null;
  identifierType: string | null;
  identifierTypeLabel: string | null;
  cardNumber: string | null;
  identifierUnavailable: boolean;
  startedAt: string;
  completedAt: string | null;
};

export type CardRead = {
  deviceId: string;
  cardType: string | null;
  cardTypeLabel: string | null;
  cardNumber: string | null;
};

export type CardReaderStatus = {
  deviceId: string;
  reader: string | null;
  readerLabel: string | null;
  readerCode: number | null;
  supported: boolean;
  cardType: string | null;
  cardTypeLabel: string | null;
  identifier: string | null;
  identifierLabel: string | null;
  mifareCustomKeyEnabled: boolean;
  hidIclassCustomKeyEnabled: boolean;
  cardCustomKeyAutoUpdate: boolean;
  message: string;
};

export type CardTestResult = {
  deviceId: string;
  outcome: string;
  message: string;
  compatible: boolean;
  reader: string | null;
  readerLabel: string | null;
  cardType: string | null;
  cardTypeLabel: string | null;
  cardNumber: string | null;
};

export type ListEnrollmentsFilter = {
  userId?: string;
  deviceId?: string;
  credentialId?: string;
  status?: EnrollmentStatus;
  limit?: number;
  offset?: number;
};
