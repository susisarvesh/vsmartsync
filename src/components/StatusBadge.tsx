import { Circle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import type { ConnectionStatus } from "@/types/devices";
import type { UserStatus } from "@/types/users";
import { cn } from "@/lib/utils";

export function StatusBadge({ status }: { status: UserStatus }) {
  const active = status === "active";
  return (
    <Badge variant={active ? "success" : "default"}>
      <Circle
        className={cn(
          "h-2.5 w-2.5 fill-current",
          active ? "text-success" : "text-muted-foreground",
        )}
        aria-hidden
      />
      <span>{active ? "Active" : "Inactive"}</span>
    </Badge>
  );
}

export function ConnectionStatusBadge({
  status,
}: {
  status: ConnectionStatus;
}) {
  const label =
    status === "online" ? "Online" : status === "offline" ? "Offline" : "Unknown";
  const variant =
    status === "online" ? "success" : status === "offline" ? "destructive" : "default";
  const iconClass =
    status === "online"
      ? "text-success"
      : status === "offline"
        ? "text-destructive"
        : "text-muted-foreground";

  return (
    <Badge variant={variant}>
      <Circle className={cn("h-2.5 w-2.5 fill-current", iconClass)} aria-hidden />
      <span>{label}</span>
    </Badge>
  );
}
