export type UserStatus = "active" | "inactive";

export type User = {
  id: string;
  name: string;
  status: UserStatus;
  createdAt: string;
  updatedAt: string;
};
