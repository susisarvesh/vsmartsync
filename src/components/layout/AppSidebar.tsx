import { CreditCard, HardDrive, LayoutDashboard, Users } from "lucide-react";
import { cn } from "@/lib/utils";
import { NAV_ITEMS, type AppRoute } from "@/navigation";

const ICONS: Record<AppRoute, typeof LayoutDashboard> = {
  dashboard: LayoutDashboard,
  users: Users,
  devices: HardDrive,
  credentials: CreditCard,
};

type AppSidebarProps = {
  route: AppRoute;
  onNavigate: (route: AppRoute) => void;
};

export function AppSidebar({ route, onNavigate }: AppSidebarProps) {
  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-card">
      <div className="border-b border-border px-3 py-3">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Navigation
        </p>
      </div>
      <nav className="flex flex-col gap-0.5 p-2" aria-label="Primary">
        {NAV_ITEMS.map((item) => {
          const Icon = ICONS[item.id];
          const active = route === item.id;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onNavigate(item.id)}
              className={cn(
                "flex items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
                active
                  ? "bg-accent text-accent-foreground"
                  : "text-foreground hover:bg-muted",
              )}
              aria-current={active ? "page" : undefined}
            >
              <Icon className="h-4 w-4 shrink-0" aria-hidden />
              <span className="font-medium">{item.label}</span>
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
