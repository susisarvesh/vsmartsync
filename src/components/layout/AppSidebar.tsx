import { useState } from "react";
import {
  ChevronDown,
  CreditCard,
  FileUp,
  HardDrive,
  LayoutDashboard,
  ScrollText,
  UserCog,
  UserPlus,
  Users,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { NAV_ITEMS, USER_NAV, USER_ROUTES, type AppRoute } from "@/navigation";

const ICONS: Record<AppRoute, typeof LayoutDashboard> = {
  dashboard: LayoutDashboard,
  "user-configuration": UserCog,
  "import-users": FileUp,
  enrollment: UserPlus,
  "in-out-report": ScrollText,
  devices: HardDrive,
  credentials: CreditCard,
};

type AppSidebarProps = {
  route: AppRoute;
  expanded: boolean;
  onNavigate: (route: AppRoute) => void;
};

export function AppSidebar({ route, expanded, onNavigate }: AppSidebarProps) {
  const usersActive = USER_ROUTES.includes(route);
  const [usersOpen, setUsersOpen] = useState(true);
  const usersExpanded = expanded && (usersOpen || usersActive);

  return (
    <aside className="flex h-full w-60 flex-col bg-card">
      <div className="flex h-14 items-center gap-2 border-b border-border px-3">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-primary text-xs font-semibold text-primary-foreground">
          V
        </span>
        <div
          className={cn(
            "min-w-0 transition-opacity duration-150",
            expanded ? "opacity-100" : "pointer-events-none opacity-0",
          )}
        >
          <p className="truncate text-sm font-semibold tracking-tight text-foreground">
            Vsmart Sync
          </p>
          <p className="truncate text-xs leading-snug text-muted-foreground">
            Local access-control workstation
          </p>
        </div>
      </div>
      <nav className="flex flex-1 flex-col gap-0.5 overflow-y-auto p-2" aria-label="Primary">
        <NavButton
          label="Dashboard"
          icon={ICONS.dashboard}
          active={route === "dashboard"}
          expanded={expanded}
          onClick={() => onNavigate("dashboard")}
        />
        <div>
          <button
            type="button"
            className={cn(
              "flex w-full items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
              usersActive
                ? "font-medium text-foreground"
                : "text-foreground hover:bg-muted",
            )}
            aria-expanded={usersExpanded}
            aria-label="Users"
            onClick={() => {
              setUsersOpen((open) => !open);
              if (!usersActive) {
                onNavigate("user-configuration");
              }
            }}
          >
            <Users className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden />
            <span
              className={cn(
                "flex-1 truncate",
                expanded ? "opacity-100" : "pointer-events-none w-0 opacity-0",
              )}
            >
              Users
            </span>
            <ChevronDown
              className={cn(
                "h-4 w-4 shrink-0 text-muted-foreground transition-transform",
                usersExpanded ? "rotate-0" : "-rotate-90",
                expanded ? "opacity-100" : "pointer-events-none w-0 opacity-0",
              )}
              aria-hidden
            />
          </button>
          {usersExpanded ? (
            <div className="ml-4 flex flex-col gap-0.5 border-l border-border pl-2">
              {USER_NAV.map((item) => (
                <NavButton
                  key={item.id}
                  label={item.label}
                  icon={ICONS[item.id]}
                  active={route === item.id}
                  expanded={expanded}
                  onClick={() => onNavigate(item.id)}
                />
              ))}
            </div>
          ) : null}
        </div>
        {NAV_ITEMS.filter((item) => item.id !== "dashboard").map((item) => (
          <NavButton
            key={item.id}
            label={item.label}
            icon={ICONS[item.id]}
            active={route === item.id}
            expanded={expanded}
            onClick={() => onNavigate(item.id)}
          />
        ))}
      </nav>
      <div className="border-t border-border px-3 py-2.5">
        <p
          className={cn(
            "text-[11px] text-muted-foreground",
            expanded ? "opacity-100" : "pointer-events-none opacity-0",
          )}
        >
          Installed modules only. Sync appears here when available.
        </p>
      </div>
    </aside>
  );
}

function NavButton({
  label,
  icon: Icon,
  active,
  expanded,
  onClick,
}: {
  label: string;
  icon: typeof LayoutDashboard;
  active: boolean;
  expanded: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "relative flex items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
        active ? "bg-accent font-medium text-accent-foreground" : "text-foreground hover:bg-muted",
      )}
      aria-label={label}
      aria-current={active ? "page" : undefined}
    >
      {active ? (
        <span className="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-primary" aria-hidden />
      ) : null}
      <Icon
        className={cn("h-4 w-4 shrink-0", active ? "text-primary" : "text-muted-foreground")}
        aria-hidden
      />
      <span className={cn("truncate", expanded ? "opacity-100" : "w-0 opacity-0")}>{label}</span>
    </button>
  );
}
