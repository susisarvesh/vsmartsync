import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { EmptyState, ErrorState, PageHeader } from "@/components/layout/PageHeader";
import { StatusBadge } from "@/components/StatusBadge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useToast } from "@/components/ui/toaster";
import { asErrorMessage as deviceErrorMessage, useDevicesQuery } from "@/hooks/useDevices";
import { asSyncErrorMessage, useSyncAssignedUsers } from "@/hooks/useSync";
import {
  asErrorMessage,
  useAssignUserDevice,
  useUsersForDeviceQuery,
  useUsersQuery,
} from "@/hooks/useUsers";
import { syncErrorMessage, type SyncUsersResult } from "@/services/sync";

type AssignDevicePageProps = {
  enabled: boolean;
};

export function AssignDevicePage({ enabled }: AssignDevicePageProps) {
  const devicesQuery = useDevicesQuery(enabled);
  const usersQuery = useUsersQuery(enabled);
  const assignUserDevice = useAssignUserDevice();
  const syncUsers = useSyncAssignedUsers();
  const { toast } = useToast();
  const [deviceId, setDeviceId] = useState("");
  const [search, setSearch] = useState("");
  const [selectedUserIds, setSelectedUserIds] = useState<string[]>([]);
  const assignedQuery = useUsersForDeviceQuery(deviceId || null);

  const selectedDevice = (devicesQuery.data ?? []).find(
    (device) => device.id === deviceId,
  );
  const assignedUserIds = useMemo(
    () => new Set((assignedQuery.data ?? []).map((row) => row.userId)),
    [assignedQuery.data],
  );

  const filteredUsers = useMemo(() => {
    const query = search.trim().toLowerCase();
    return (usersQuery.data ?? []).filter((user) => {
      if (!query) {
        return true;
      }
      return user.username.toLowerCase().includes(query);
    });
  }, [search, usersQuery.data]);

  function userOnDevice(userId: string) {
    return (assignedQuery.data ?? []).some(
      (row) => row.userId === userId && row.provisionedAt,
    );
  }

  const selectableIds = filteredUsers
    .filter((user) => !assignedUserIds.has(user.id))
    .map((user) => user.id);
  const allSelectableChecked =
    selectableIds.length > 0 &&
    selectableIds.every((id) => selectedUserIds.includes(id));

  function toggleUser(userId: string) {
    setSelectedUserIds((current) =>
      current.includes(userId)
        ? current.filter((id) => id !== userId)
        : [...current, userId],
    );
  }

  function toggleAllSelectable() {
    setSelectedUserIds((current) => {
      if (allSelectableChecked) {
        return current.filter((id) => !selectableIds.includes(id));
      }
      return Array.from(new Set([...current, ...selectableIds]));
    });
  }

  async function onAssign() {
    if (!deviceId || selectedUserIds.length === 0 || !selectedDevice) {
      return;
    }
    const pending = selectedUserIds.filter((id) => !assignedUserIds.has(id));
    let assigned = 0;
    const assignedIds: string[] = [];
    let firstError: unknown = null;
    for (const userId of pending) {
      try {
        await assignUserDevice.mutateAsync({ userId, deviceId });
        assigned += 1;
        assignedIds.push(userId);
      } catch (reason: unknown) {
        firstError ??= reason;
      }
    }
    setSelectedUserIds([]);
    if (assigned > 0) {
      toast({
        title: "Users assigned",
        description: `${assigned} assigned to ${selectedDevice.deviceName}.`,
        variant: "success",
      });
    }
    if (firstError) {
      toast({
        title: "Some users were not assigned",
        description: asErrorMessage(firstError),
        variant: "destructive",
      });
    }
    if (assignedIds.length > 0) {
      await pushUsers(assignedIds, selectedDevice.deviceName);
    }
  }

  async function onSyncAssigned() {
    if (!deviceId || !selectedDevice) {
      return;
    }
    const ids = (assignedQuery.data ?? []).map((row) => row.userId);
    await pushUsers(ids, selectedDevice.deviceName);
  }

  async function pushUsers(userIds: string[], deviceName: string) {
    try {
      const result = await syncUsers.mutateAsync({ deviceId, userIds });
      reportSync(result, deviceName);
    } catch (reason: unknown) {
      toast({
        title: "Could not add users on the device",
        description: asSyncErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  function reportSync(result: SyncUsersResult, deviceName: string) {
    if (result.synced.length > 0) {
      toast({
        title: "Users added on the device",
        description: `${result.synced.length} added on ${deviceName}.`,
        variant: "success",
      });
    }
    if (result.failed.length > 0) {
      toast({
        title: "Some users were not added on the device",
        description: syncErrorMessage(result.failed[0]?.code ?? ""),
        variant: "destructive",
      });
    }
  }

  if (!enabled) {
    return (
      <div>
        <PageHeader
          title="Assign to device"
          description="Select a device, then choose users to assign and add on that COSEC device."
        />
        <ErrorState
          title="Database required"
          description="Connect local PostgreSQL from Dashboard before assigning users."
        />
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Assign to device"
        description="Select a device, then choose users. Assign stores the link locally and adds each person on the COSEC device."
        action={
          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="secondary"
              disabled={
                !deviceId ||
                (assignedQuery.data?.length ?? 0) === 0 ||
                syncUsers.isPending ||
                assignUserDevice.isPending
              }
              onClick={() => void onSyncAssigned()}
            >
              {syncUsers.isPending ? "Syncing…" : "Sync assigned"}
            </Button>
            <Button
              type="button"
              disabled={
                !deviceId ||
                selectedUserIds.length === 0 ||
                assignUserDevice.isPending ||
                syncUsers.isPending
              }
              onClick={() => void onAssign()}
            >
              {assignUserDevice.isPending || syncUsers.isPending
                ? "Working…"
                : `Assign and sync${selectedUserIds.length > 0 ? ` (${selectedUserIds.length})` : ""}`}
            </Button>
          </div>
        }
      />

      <div className="mb-4 grid max-w-md gap-2">
        <Label htmlFor="assign-target-device">Device</Label>
        <Select
          value={deviceId || undefined}
          onValueChange={(value) => {
            setDeviceId(value);
            setSelectedUserIds([]);
          }}
        >
          <SelectTrigger id="assign-target-device" aria-label="Device">
            <SelectValue placeholder="Select a device" />
          </SelectTrigger>
          <SelectContent>
            {(devicesQuery.data ?? []).map((device) => (
              <SelectItem key={device.id} value={device.id}>
                {device.deviceName} ({device.host}:{device.port})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {devicesQuery.isLoading ? (
        <p className="text-sm text-muted-foreground">Loading devices…</p>
      ) : null}
      {devicesQuery.isError ? (
        <ErrorState
          title="Unable to load devices"
          description={deviceErrorMessage(devicesQuery.error)}
          onRetry={() => void devicesQuery.refetch()}
        />
      ) : null}
      {!devicesQuery.isLoading &&
      !devicesQuery.isError &&
      (devicesQuery.data?.length ?? 0) === 0 ? (
        <EmptyState
          title="No devices yet"
          description="Register a device on the Devices page before assigning users."
        />
      ) : null}

      {deviceId ? (
        <>
          <div className="relative mb-3 max-w-sm">
            <Search
              className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
              aria-hidden
            />
            <Input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search users…"
              className="pl-8"
              aria-label="Search users"
            />
          </div>

          {usersQuery.isLoading || assignedQuery.isLoading ? (
            <div className="rounded-lg border border-border bg-card px-4 py-6 text-sm text-muted-foreground">
              Loading users…
            </div>
          ) : null}
          {usersQuery.isError ? (
            <ErrorState
              title="Unable to load users"
              description={asErrorMessage(usersQuery.error)}
              onRetry={() => void usersQuery.refetch()}
            />
          ) : null}
          {assignedQuery.isError ? (
            <ErrorState
              title="Unable to load assignments"
              description={asErrorMessage(assignedQuery.error)}
              onRetry={() => void assignedQuery.refetch()}
            />
          ) : null}

          {!usersQuery.isLoading &&
          !usersQuery.isError &&
          (usersQuery.data?.length ?? 0) === 0 ? (
            <EmptyState
              title="No users yet"
              description="Create a user on the Users page before assigning anyone."
            />
          ) : null}

          {!usersQuery.isLoading &&
          !assignedQuery.isLoading &&
          !usersQuery.isError &&
          !assignedQuery.isError &&
          (usersQuery.data?.length ?? 0) > 0 &&
          filteredUsers.length === 0 ? (
            <EmptyState
              title="No matching users"
              description="Try a different search."
            />
          ) : null}

          {filteredUsers.length > 0 &&
          !usersQuery.isError &&
          !assignedQuery.isError &&
          !assignedQuery.isLoading ? (
            <div className="overflow-x-auto rounded-lg border border-border bg-card">
              <table className="w-full border-collapse text-sm">
                <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
                  <tr>
                    <th className="w-10 px-3 py-2 font-medium">
                      <input
                        type="checkbox"
                        aria-label="Select all users not yet assigned"
                        checked={allSelectableChecked}
                        disabled={selectableIds.length === 0}
                        onChange={toggleAllSelectable}
                      />
                    </th>
                    <th className="px-3 py-2 font-medium">Username</th>
                    <th className="px-3 py-2 font-medium">Status</th>
                    <th className="px-3 py-2 font-medium">Assignment</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredUsers.map((user) => {
                    const alreadyAssigned = assignedUserIds.has(user.id);
                    const checked =
                      alreadyAssigned || selectedUserIds.includes(user.id);
                    return (
                      <tr key={user.id} className="border-t border-border">
                        <td className="px-3 py-2">
                          <input
                            type="checkbox"
                            aria-label={`Select ${user.username}`}
                            checked={checked}
                            disabled={
                              alreadyAssigned ||
                              assignUserDevice.isPending ||
                              syncUsers.isPending
                            }
                            onChange={() => toggleUser(user.id)}
                          />
                        </td>
                        <td className="px-3 py-2 font-medium text-foreground">
                          {user.username}
                        </td>
                        <td className="px-3 py-2">
                          <StatusBadge status={user.status} />
                        </td>
                        <td className="px-3 py-2 text-muted-foreground">
                          {alreadyAssigned
                            ? userOnDevice(user.id)
                              ? "On device"
                              : "Assigned locally"
                            : "Not assigned"}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
