export type AppRoute =
  | "dashboard"
  | "users"
  | "devices"
  | "credentials"
  | "enrollments";

export const NAV_ITEMS: Array<{
  id: AppRoute;
  label: string;
  description: string;
}> = [
  {
    id: "dashboard",
    label: "Dashboard",
    description: "Operational status for this workstation",
  },
  {
    id: "users",
    label: "Users",
    description: "Manage users registered in the system",
  },
  {
    id: "devices",
    label: "Devices",
    description: "Register and reach Matrix COSEC devices",
  },
  {
    id: "credentials",
    label: "Credentials",
    description: "Manage user Card and PIN credentials",
  },
  {
    id: "enrollments",
    label: "Enrollments",
    description: "Assign credentials to devices",
  },
];
