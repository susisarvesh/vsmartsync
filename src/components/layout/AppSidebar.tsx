import { CreditCard, HardDrive, LayoutDashboard, Link2, Users } from "lucide-react";
import { cn } from "@/lib/utils";
import { NAV_ITEMS, type AppRoute } from "@/navigation";

const ICONS: Record<AppRoute, typeof LayoutDashboard> = {
  dashboard: LayoutDashboard,
  users: Users,
  devices: HardDrive,
  credentials: CreditCard,
  enrollments: Link2,
};

type AppSidebarProps = {
  route: AppRoute;
  onNavigate: (route: AppRoute) => void;
};

export function AppSidebar({ route, onNavigate }: AppSidebarProps) {
  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-border bg-card">
      <div className="border-b border-border px-4 py-3.5">
        <p className="text-sm font-semibold tracking-tight text-foreground">
          Vsmart Sync
        </p>
        <p className="mt-0.5 text-xs leading-snug text-muted-foreground">
          Local access-control workstation
        </p>
      </div>
      <nav className="flex flex-1 flex-col gap-0.5 p-2" aria-label="Primary">
        {NAV_ITEMS.map((item) => {
          const Icon = ICONS[item.id];
          const active = route === item.id;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onNavigate(item.id)}
              className={cn(
                "relative flex items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
                active
                  ? "bg-accent font-medium text-accent-foreground"
                  : "text-foreground hover:bg-muted",
              )}
              aria-current={active ? "page" : undefined}
            >
              {active ? (
                <span
                  className="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-primary"
                  aria-hidden
                />
              ) : null}
              <Icon
                className={cn(
                  "h-4 w-4 shrink-0",
                  active ? "text-primary" : "text-muted-foreground",
                )}
                aria-hidden
              />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>
      <div className="border-t border-border px-3 py-2.5">
        <p className="text-[11px] text-muted-foreground">
          Installed modules only. Sync and events appear here when available.
        </p>
      </div>
    </aside>
  );
}
