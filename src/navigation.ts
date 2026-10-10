export type AppRoute =
  | "dashboard"
  | "user-configuration"
  | "import-users"
  | "enrollment"
  | "in-out-report"
  | "devices"
  | "credentials";

export const USER_ROUTES: AppRoute[] = [
  "user-configuration",
  "import-users",
  "enrollment",
  "in-out-report",
];

export const USER_NAV: Array<{
  id: AppRoute;
  label: string;
  description: string;
}> = [
  {
    id: "user-configuration",
    label: "User Configuration",
    description: "Identity and status of a person",
  },
  {
    id: "import-users",
    label: "Import User",
    description: "Add people from an Excel list",
  },
  {
    id: "enrollment",
    label: "Enrollment",
    description: "Assign a person to a device and capture a card or face",
  },
  {
    id: "in-out-report",
    label: "In/Out Report",
    description: "Card and face entries reported by a device",
  },
];

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
    id: "devices",
    label: "Devices",
    description: "Register and reach Matrix COSEC devices",
  },
  {
    id: "credentials",
    label: "Credentials",
    description: "Credentials for a user, including enrollment on a device",
  },
];
