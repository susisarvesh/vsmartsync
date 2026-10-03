import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { MoreHorizontal, Plus, Search } from "lucide-react";
import { PageHeader, EmptyState, ErrorState } from "@/components/layout/PageHeader";
import { HardwareEnrollDialog } from "@/components/HardwareEnrollDialog";
import { StatusBadge } from "@/components/StatusBadge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
import { useDevicesQuery } from "@/hooks/useDevices";
import { asSyncErrorMessage, useSyncAssignedUsers } from "@/hooks/useSync";
import { syncErrorMessage } from "@/services/sync";
import {
  asErrorMessage,
  useActivateUser,
  useAssignUserDevice,
  useCreateUser,
  useDeactivateUser,
  useDeleteUser,
  useRemoveUserDevice,
  useUpdateUser,
  useUserDevicesQuery,
  useUsersQuery,
} from "@/hooks/useUsers";
import { formatDateTime } from "@/lib/utils";
import type { User, UserDeviceAssignment } from "@/types/users";

const usernameSchema = z.object({
  username: z
    .string()
    .trim()
    .min(1, "Username must contain at least 1 character.")
    .max(200, "Username must be 200 characters or fewer."),
});

type UsernameForm = z.infer<typeof usernameSchema>;

type UsersPageProps = {
  enabled: boolean;
};

export function UsersPage({ enabled }: UsersPageProps) {
  const { toast } = useToast();
  const usersQuery = useUsersQuery(enabled);
  const createUser = useCreateUser();
  const updateUser = useUpdateUser();
  const activateUser = useActivateUser();
  const deactivateUser = useDeactivateUser();
  const deleteUser = useDeleteUser();
  const devicesQuery = useDevicesQuery(enabled);
  const assignUserDevice = useAssignUserDevice();
  const syncUsers = useSyncAssignedUsers();
  const removeUserDevice = useRemoveUserDevice();

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all" | "active" | "inactive">(
    "all",
  );
  const [createOpen, setCreateOpen] = useState(false);
  const [viewUser, setViewUser] = useState<User | null>(null);
  const [enrollUser, setEnrollUser] = useState<User | null>(null);
  const [editUser, setEditUser] = useState<User | null>(null);
  const [activateTarget, setActivateTarget] = useState<User | null>(null);
  const [deactivateTarget, setDeactivateTarget] = useState<User | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<User | null>(null);
  const [assignUser, setAssignUser] = useState<User | null>(null);
  const [selectedDeviceId, setSelectedDeviceId] = useState("");
  const [removeTarget, setRemoveTarget] = useState<UserDeviceAssignment | null>(
    null,
  );

  const createForm = useForm<UsernameForm>({
    resolver: zodResolver(usernameSchema),
    defaultValues: { username: "" },
  });
  const assignmentUserId = assignUser?.id ?? viewUser?.id ?? null;
  const assignmentsQuery = useUserDevicesQuery(assignmentUserId);

  const editForm = useForm<UsernameForm>({
    resolver: zodResolver(usernameSchema),
    defaultValues: { username: "" },
  });

  const filtered = useMemo(() => {
    const users = usersQuery.data ?? [];
    const query = search.trim().toLowerCase();
    return users.filter((user) => {
      if (statusFilter !== "all" && user.status !== statusFilter) {
        return false;
      }
      if (!query) {
        return true;
      }
      return user.username.toLowerCase().includes(query);
    });
  }, [usersQuery.data, search, statusFilter]);

  async function onCreate(values: UsernameForm) {
    try {
      await createUser.mutateAsync(values.username);
      toast({
        title: "User created successfully",
        variant: "success",
      });
      createForm.reset({ username: "" });
      setCreateOpen(false);
    } catch (reason: unknown) {
      toast({
        title: "Could not create user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onEdit(values: UsernameForm) {
    if (!editUser) {
      return;
    }
    try {
      await updateUser.mutateAsync({ id: editUser.id, username: values.username });
      toast({
        title: "User updated successfully",
        variant: "success",
      });
      setEditUser(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not update user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onConfirmActivate() {
    if (!activateTarget) {
      return;
    }
    try {
      await activateUser.mutateAsync(activateTarget.id);
      toast({
        title: "User activated",
        description: `${activateTarget.username} is now active.`,
        variant: "success",
      });
      setActivateTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not activate user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onConfirmDeactivate() {
    if (!deactivateTarget) {
      return;
    }
    try {
      await deactivateUser.mutateAsync(deactivateTarget.id);
      toast({
        title: "User deactivated",
        description: `${deactivateTarget.username} is now inactive.`,
        variant: "success",
      });
      setDeactivateTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not deactivate user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onConfirmDelete() {
    if (!deleteTarget) {
      return;
    }
    try {
      await deleteUser.mutateAsync(deleteTarget.id);
      toast({
        title: "User deleted",
        description: `${deleteTarget.username} was removed here and on each device that had this user.`,
        variant: "success",
      });
      setDeleteTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not delete user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onAssignDevice() {
    if (!assignUser || !selectedDeviceId) {
      return;
    }
    try {
      await assignUserDevice.mutateAsync({
        userId: assignUser.id,
        deviceId: selectedDeviceId,
      });
      const deviceName =
        (devicesQuery.data ?? []).find((device) => device.id === selectedDeviceId)
          ?.deviceName ?? "the device";
      toast({
        title: "Device assigned",
        description: `${assignUser.username} is assigned to ${deviceName}.`,
        variant: "success",
      });
      try {
        const result = await syncUsers.mutateAsync({
          deviceId: selectedDeviceId,
          userIds: [assignUser.id],
        });
        if (result.synced.length > 0) {
          toast({
            title: "User added on the device",
            description: `${assignUser.username} was added on ${deviceName}.`,
            variant: "success",
          });
        }
        if (result.failed.length > 0) {
          toast({
            title: "User was not added on the device",
            description: syncErrorMessage(result.failed[0]?.code ?? ""),
            variant: "destructive",
          });
        }
      } catch (reason: unknown) {
        toast({
          title: "User was not added on the device",
          description: asSyncErrorMessage(reason),
          variant: "destructive",
        });
      }
      setSelectedDeviceId("");
    } catch (reason: unknown) {
      toast({
        title: "Could not assign device",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onConfirmRemoveDevice() {
    if (!assignUser || !removeTarget) {
      return;
    }
    try {
      await removeUserDevice.mutateAsync({
        userId: assignUser.id,
        deviceId: removeTarget.deviceId,
      });
      toast({
        title: "Assignment removed",
        description: `${assignUser.username} is no longer assigned to ${removeTarget.deviceName}.`,
        variant: "success",
      });
      setRemoveTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not remove assignment",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  const assignedDeviceIds = new Set(
    (assignmentsQuery.data ?? []).map((assignment) => assignment.deviceId),
  );
  const availableDevices = (devicesQuery.data ?? []).filter(
    (device) => !assignedDeviceIds.has(device.id),
  );

  if (!enabled) {
    return (
      <div>
        <PageHeader
          title="Users"
          description="Manage users registered in the system."
        />
        <ErrorState
          title="Database required"
          description="Connect local PostgreSQL from Dashboard before managing users."
        />
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Users"
        description="Create, view, and update people. New users start Active. Delete removes the person here and on every Matrix device that already has them."
        action={
          <Button type="button" onClick={() => setCreateOpen(true)}>
            <Plus className="h-4 w-4" aria-hidden />
            Add User
          </Button>
        }
      />

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <div className="relative min-w-[14rem] flex-1">
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
        <Select
          value={statusFilter}
          onValueChange={(value: "all" | "active" | "inactive") =>
            setStatusFilter(value)
          }
        >
          <SelectTrigger className="w-36" aria-label="Filter by status">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All statuses</SelectItem>
            <SelectItem value="active">Active</SelectItem>
            <SelectItem value="inactive">Inactive</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {usersQuery.isLoading ? (
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

      {!usersQuery.isLoading &&
      !usersQuery.isError &&
      (usersQuery.data?.length ?? 0) === 0 ? (
        <EmptyState
          title="No users yet"
          description="Add your first user to begin managing people in the system of record."
          actionLabel="Add User"
          onAction={() => setCreateOpen(true)}
        />
      ) : null}

      {!usersQuery.isLoading &&
      !usersQuery.isError &&
      (usersQuery.data?.length ?? 0) > 0 &&
      filtered.length === 0 ? (
        <EmptyState
          title="No matching users"
          description="Try a different search or clear the status filter."
        />
      ) : null}

      {filtered.length > 0 ? (
        <div className="overflow-x-auto rounded-lg border border-border bg-card">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">Username</th>
                <th className="px-3 py-2 font-medium">Status</th>
                <th className="px-3 py-2 font-medium">Created</th>
                <th className="px-3 py-2 font-medium">Updated</th>
                <th className="px-3 py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((user) => (
                <tr key={user.id} className="border-t border-border">
                  <td className="px-3 py-2 font-medium text-foreground">
                    {user.username}
                  </td>
                  <td className="px-3 py-2">
                    <StatusBadge status={user.status} />
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {formatDateTime(user.createdAt)}
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {formatDateTime(user.updatedAt)}
                  </td>
                  <td className="px-3 py-2 text-right">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={`Actions for ${user.username}`}
                        >
                          <MoreHorizontal className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onSelect={() => setViewUser(user)}>
                          View
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onSelect={() => {
                            setEditUser(user);
                            editForm.reset({ username: user.username });
                          }}
                        >
                          Edit username
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onSelect={() => {
                            setAssignUser(user);
                            setSelectedDeviceId("");
                          }}
                        >
                          Devices
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          disabled={user.status === "active"}
                          onSelect={() => setActivateTarget(user)}
                        >
                          Activate
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          destructive
                          disabled={user.status === "inactive"}
                          onSelect={() => setDeactivateTarget(user)}
                        >
                          Deactivate
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          destructive
                          onSelect={() => setDeleteTarget(user)}
                        >
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add user</DialogTitle>
            <DialogDescription>
              Creates a person record in the local database. The user starts as
              Active.
            </DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-3"
            onSubmit={createForm.handleSubmit(onCreate)}
          >
            <div className="grid gap-1.5">
              <Label htmlFor="create-user-username">Username</Label>
              <Input
                id="create-user-username"
                autoComplete="off"
                maxLength={200}
                {...createForm.register("username")}
              />
              {createForm.formState.errors.username ? (
                <p className="text-xs text-destructive">
                  {createForm.formState.errors.username.message}
                </p>
              ) : null}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setCreateOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={createUser.isPending}>
                {createUser.isPending ? "Saving…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(viewUser)}
        onOpenChange={(open) => {
          if (!open) {
            setViewUser(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{viewUser?.username ?? "User"}</DialogTitle>
            <DialogDescription>
              Local user record. Credentials are managed on the Credentials
              page. Device assignment does not create the user on the device.
            </DialogDescription>
          </DialogHeader>
          <dl className="grid gap-2 text-sm">
            <div>
              <dt className="text-muted-foreground">ID</dt>
              <dd className="font-mono text-xs">{viewUser?.id}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Username</dt>
              <dd>{viewUser?.username}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Status</dt>
              <dd>{viewUser ? <StatusBadge status={viewUser.status} /> : null}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Created</dt>
              <dd>{viewUser ? formatDateTime(viewUser.createdAt) : null}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Updated</dt>
              <dd>{viewUser ? formatDateTime(viewUser.updatedAt) : null}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Assigned devices</dt>
              <dd>
                {assignmentsQuery.isLoading ? "Loading…" : null}
                {!assignmentsQuery.isLoading &&
                (assignmentsQuery.data?.length ?? 0) === 0
                  ? "None"
                  : null}
                {(assignmentsQuery.data ?? []).map((assignment) => (
                  <div key={assignment.id}>
                    {assignment.deviceName} ({assignment.host}:{assignment.port})
                  </div>
                ))}
              </dd>
            </div>
          </dl>
          <DialogFooter>
            {viewUser?.status === "active" ? (
              <Button
                type="button"
                onClick={() => {
                  setEnrollUser(viewUser);
                  setViewUser(null);
                }}
              >
                Enroll credential
              </Button>
            ) : null}
            <Button type="button" variant="secondary" onClick={() => setViewUser(null)}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(assignUser)}
        onOpenChange={(open) => {
          if (!open) {
            setAssignUser(null);
            setSelectedDeviceId("");
            setRemoveTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Devices for {assignUser?.username}</DialogTitle>
            <DialogDescription>
              Assign this person to one or more devices. Each assignment is
              also added on that COSEC device.
            </DialogDescription>
          </DialogHeader>
          {assignmentsQuery.isLoading ? (
            <p className="text-sm text-muted-foreground">Loading assignments…</p>
          ) : null}
          {assignmentsQuery.isError ? (
            <p className="text-sm text-destructive">
              {asErrorMessage(assignmentsQuery.error)}
            </p>
          ) : null}
          {!assignmentsQuery.isLoading &&
          (assignmentsQuery.data?.length ?? 0) === 0 ? (
            <p className="text-sm text-muted-foreground">
              Not assigned to any device yet.
            </p>
          ) : null}
          {(assignmentsQuery.data ?? []).length > 0 ? (
            <ul className="grid gap-2 text-sm">
              {(assignmentsQuery.data ?? []).map((assignment) => (
                <li
                  key={assignment.id}
                  className="flex items-center justify-between gap-2"
                >
                  <span>
                    {assignment.deviceName} ({assignment.host}:{assignment.port})
                  </span>
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    disabled={removeUserDevice.isPending}
                    onClick={() => setRemoveTarget(assignment)}
                  >
                    Remove
                  </Button>
                </li>
              ))}
            </ul>
          ) : null}
          <div className="grid gap-2">
            <Label htmlFor="assign-device">Add device</Label>
            <Select
              value={selectedDeviceId || undefined}
              onValueChange={setSelectedDeviceId}
            >
              <SelectTrigger id="assign-device" aria-label="Device to assign">
                <SelectValue placeholder="Select a device" />
              </SelectTrigger>
              <SelectContent>
                {availableDevices.map((device) => (
                  <SelectItem key={device.id} value={device.id}>
                    {device.deviceName} ({device.host}:{device.port})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {availableDevices.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                Every registered device is already assigned, or no devices exist
                yet.
              </p>
            ) : null}
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setAssignUser(null)}
            >
              Close
            </Button>
            <Button
              type="button"
              disabled={!selectedDeviceId || assignUserDevice.isPending}
              onClick={() => void onAssignDevice()}
            >
              {assignUserDevice.isPending || syncUsers.isPending
                ? "Working…"
                : "Assign and sync"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(removeTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setRemoveTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Remove device assignment?</DialogTitle>
            <DialogDescription>
              This removes{" "}
              <strong>{removeTarget?.deviceName}</strong> from{" "}
              <strong>{assignUser?.username}</strong> in the local database.
              The user and the device stay. It does not delete the person from
              the Matrix device. You can assign them again later.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setRemoveTarget(null)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={removeUserDevice.isPending}
              onClick={() => void onConfirmRemoveDevice()}
            >
              {removeUserDevice.isPending ? "Working…" : "Remove assignment"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(editUser)}
        onOpenChange={(open) => {
          if (!open) {
            setEditUser(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit username</DialogTitle>
            <DialogDescription>
              Changes the username only. Use Activate or Deactivate to change
              status.
            </DialogDescription>
          </DialogHeader>
          <form className="grid gap-3" onSubmit={editForm.handleSubmit(onEdit)}>
            <div className="grid gap-1.5">
              <Label htmlFor="edit-user-username">Username</Label>
              <Input
                id="edit-user-username"
                autoComplete="off"
                maxLength={200}
                {...editForm.register("username")}
              />
              {editForm.formState.errors.username ? (
                <p className="text-xs text-destructive">
                  {editForm.formState.errors.username.message}
                </p>
              ) : null}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setEditUser(null)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={updateUser.isPending}>
                {updateUser.isPending ? "Saving…" : "Save"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(activateTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setActivateTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Activate user?</DialogTitle>
            <DialogDescription>
              This marks <strong>{activateTarget?.username}</strong> as active
              in the local database. It does not create the user on a Matrix
              device.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setActivateTarget(null)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              disabled={activateUser.isPending}
              onClick={() => void onConfirmActivate()}
            >
              {activateUser.isPending ? "Working…" : "Activate user"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(deactivateTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setDeactivateTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Deactivate user?</DialogTitle>
            <DialogDescription>
              This marks <strong>{deactivateTarget?.username}</strong> as
              inactive in the local database. The record is kept. It does not
              delete the user from a Matrix device. You can activate them again
              later.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setDeactivateTarget(null)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={deactivateUser.isPending}
              onClick={() => void onConfirmDeactivate()}
            >
              {deactivateUser.isPending ? "Working…" : "Deactivate user"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(deleteTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete user?</DialogTitle>
            <DialogDescription>
              This removes <strong>{deleteTarget?.username}</strong> from Vsmart
              Sync and from every Matrix device that already has this user,
              including their enrollments. If a device cannot be reached, the
              user is kept.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setDeleteTarget(null)}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={deleteUser.isPending}
              onClick={() => void onConfirmDelete()}
            >
              {deleteUser.isPending ? "Deleting…" : "Delete user"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <HardwareEnrollDialog
        open={Boolean(enrollUser)}
        onOpenChange={(open) => {
          if (!open) {
            setEnrollUser(null);
          }
        }}
        fixedUser={
          enrollUser
            ? { id: enrollUser.id, username: enrollUser.username }
            : null
        }
      />
    </div>
  );
}
