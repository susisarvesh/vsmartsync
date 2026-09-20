export type CredentialType = "card" | "pin";
export type CredentialStatus = "active" | "inactive";

export type Credential = {
  id: string;
  userId: string;
  userName: string;
  credentialType: CredentialType;
  status: CredentialStatus;
  maskedValue: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CreateCredentialInput = {
  userId: string;
  credentialType: CredentialType;
  value: string;
};

export type ListCredentialsFilter = {
  userId?: string;
  credentialType?: CredentialType;
  status?: CredentialStatus;
};
