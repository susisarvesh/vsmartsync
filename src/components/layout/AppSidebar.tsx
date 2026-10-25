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
    <aside className="flex h-full w-full flex-col">
      <div className="shrink-0 overflow-hidden border-b border-slate-200 bg-white">
        <button
          type="button"
          onClick={() => onNavigate("dashboard")}
          className="flex h-16 w-full items-center justify-center px-2"
          aria-label="Go to Dashboard"
        >
          {expanded ? (
            <img
              src="/vsmart-logo-full.png"
              alt="Vsmart Technologies"
              className="h-12 w-auto max-w-[13.5rem] object-contain"
              width={216}
              height={52}
            />
          ) : (
            <img
              src="/favicon.png"
              alt=""
              className="h-9 w-9 object-contain"
              width={36}
              height={36}
            />
          )}
        </button>
      </div>
      <nav
        className={cn(
          "flex flex-1 flex-col overflow-y-auto",
          expanded ? "gap-1 p-3" : "items-center gap-2 px-2 py-3",
        )}
        aria-label="Primary"
      >
        <NavButton
          label="Dashboard"
          icon={ICONS.dashboard}
          active={route === "dashboard"}
          expanded={expanded}
          onClick={() => onNavigate("dashboard")}
        />
        <div className={cn(expanded ? "w-full" : "flex w-full justify-center")}>
          <button
            type="button"
            className={cn(
              "flex items-center rounded-lg text-sm transition-colors",
              expanded ? "w-full gap-3 px-3 py-2.5 text-left" : "h-10 w-10 justify-center",
              usersActive
                ? "bg-blue-500 font-medium text-white"
                : "text-slate-200 hover:bg-white/10 hover:text-white",
            )}
            aria-expanded={usersExpanded}
            aria-label="Users"
            title={expanded ? undefined : "Users"}
            onClick={() => {
              if (!expanded) {
                onNavigate("user-configuration");
                return;
              }
              setUsersOpen((open) => !open);
              if (!usersActive) {
                onNavigate("user-configuration");
              }
            }}
          >
            <Users className="h-5 w-5 shrink-0" aria-hidden />
            {expanded ? <span className="min-w-0 flex-1 truncate">Users</span> : null}
            {expanded ? (
              <ChevronDown
                className={cn(
                  "h-4 w-4 shrink-0 transition-transform",
                  usersExpanded ? "rotate-180" : "rotate-0",
                )}
                aria-hidden
              />
            ) : null}
          </button>
          {usersExpanded ? (
            <div className="my-1 ml-4 flex flex-col gap-0.5 border-l border-white/15 pl-2">
              {USER_NAV.map((item) => (
                <NavButton
                  key={item.id}
                  label={item.label}
                  icon={ICONS[item.id]}
                  active={route === item.id}
                  expanded={expanded}
                  nested
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
    </aside>
  );
}

function NavButton({
  label,
  icon: Icon,
  active,
  expanded,
  nested = false,
  onClick,
}: {
  label: string;
  icon: typeof LayoutDashboard;
  active: boolean;
  expanded: boolean;
  nested?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex items-center rounded-lg text-sm transition-colors",
        expanded
          ? cn("w-full gap-3 px-3 text-left", nested ? "py-2" : "py-2.5")
          : "h-10 w-10 justify-center",
        active
          ? nested
            ? "bg-blue-400 font-medium text-white"
            : "bg-blue-500 font-medium text-white"
          : "text-slate-200 hover:bg-white/10 hover:text-white",
      )}
      aria-label={label}
      aria-current={active ? "page" : undefined}
      title={expanded ? undefined : label}
    >
      {nested ? (
        <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-current" aria-hidden />
      ) : (
        <Icon className="h-5 w-5 shrink-0" aria-hidden />
      )}
      {expanded ? <span className="min-w-0 truncate">{label}</span> : null}
    </button>
  );
}
