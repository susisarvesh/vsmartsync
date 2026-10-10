export type AppRoute =
  | "dashboard"
  | "users"
  | "assign-device"
  | "devices"
  | "credentials"
  | "access-logs";

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
    id: "assign-device",
    label: "Assign to device",
    description: "Choose a device, then assign users to it",
  },
  {
    id: "devices",
    label: "Devices",
    description: "Register and reach Matrix COSEC devices",
  },
  {
    id: "credentials",
    label: "Credentials",
    description: "Credentials for a user, including enrollment on a device",
  },
  {
    id: "access-logs",
    label: "Access logs",
    description: "Card and face entries reported by a device",
  },
];
