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
              className="h-[3.25rem] w-auto max-w-[13.5rem] object-contain"
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
              "flex w-full items-center rounded-lg py-2.5 text-left text-sm transition-colors",
              expanded ? "gap-3 px-3" : "justify-center px-0",
              usersActive
                ? "bg-blue-500 font-medium text-white shadow-md"
                : "text-slate-200 hover:bg-white/10 hover:text-white",
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
            <Users className="h-5 w-5 shrink-0" aria-hidden />
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
                "h-4 w-4 shrink-0 transition-transform",
                usersExpanded ? "rotate-180" : "rotate-0",
                expanded ? "opacity-100" : "pointer-events-none w-0 opacity-0",
              )}
              aria-hidden
            />
          </button>
          {usersExpanded ? (
            <div className="my-1 ml-2 flex flex-col gap-0.5 border-l border-white/15 pl-2">
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
        "flex w-full items-center rounded-lg text-left text-sm transition-colors",
        nested ? "gap-3 px-3 py-2" : expanded ? "mb-0.5 gap-3 px-3 py-2.5" : "mb-0.5 justify-center px-0 py-2.5",
        active
          ? nested
            ? "bg-blue-400 font-medium text-white shadow-sm"
            : "bg-blue-500 font-medium text-white shadow-md"
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
      <span className={cn("truncate", expanded ? "opacity-100" : "w-0 opacity-0")}>{label}</span>
    </button>
  );
}
