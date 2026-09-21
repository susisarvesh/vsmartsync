import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { MoreHorizontal, Plus, Search } from "lucide-react";
import { PageHeader, EmptyState, ErrorState } from "@/components/layout/PageHeader";
import { Badge } from "@/components/ui/badge";
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
import {
  asErrorMessage,
  useCancelEnrollment,
  useCreateEnrollment,
  useEnrollmentsQuery,
  useRetryEnrollment,
  useRevokeEnrollment,
} from "@/hooks/useEnrollments";
import { useCredentialsQuery } from "@/hooks/useCredentials";
import { useUsersQuery } from "@/hooks/useUsers";
import { formatDateTime } from "@/lib/utils";
import type { Enrollment, EnrollmentStatus } from "@/types/enrollments";

const createSchema = z.object({
  userId: z.string().uuid("Select a user."),
  credentialId: z.string().uuid("Select a credential."),
  deviceId: z.string().uuid("Select a device."),
});

type CreateForm = z.infer<typeof createSchema>;

type EnrollmentsPageProps = {
  enabled: boolean;
};

function statusLabel(status: EnrollmentStatus): string {
  return status.charAt(0).toUpperCase() + status.slice(1);
}

function EnrollmentStatusBadge({ status }: { status: EnrollmentStatus }) {
  const variant =
    status === "active"
      ? "success"
      : status === "failed" || status === "revoked"
        ? "destructive"
        : status === "pending"
          ? "warning"
          : "default";
  return (
    <Badge variant={variant}>
      <span>{statusLabel(status)}</span>
    </Badge>
  );
}

function credentialLabel(type: string, masked: string | null): string {
  if (type === "pin") {
    return "PIN · Configured";
  }
  return `Card · ${masked ?? "Configured"}`;
}

export function EnrollmentsPage({ enabled }: EnrollmentsPageProps) {
  const { toast } = useToast();
  const usersQuery = useUsersQuery(enabled);
  const devicesQuery = useDevicesQuery(enabled);
  const [search, setSearch] = useState("");
  const [userFilter, setUserFilter] = useState("all");
  const [deviceFilter, setDeviceFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState<"all" | EnrollmentStatus>(
    "all",
  );
  const [createOpen, setCreateOpen] = useState(false);
  const [cancelTarget, setCancelTarget] = useState<Enrollment | null>(null);
  const [revokeTarget, setRevokeTarget] = useState<Enrollment | null>(null);

  const listFilter = useMemo(
    () => ({
      userId: userFilter === "all" ? undefined : userFilter,
      deviceId: deviceFilter === "all" ? undefined : deviceFilter,
      status: statusFilter === "all" ? undefined : statusFilter,
      limit: 200,
    }),
    [userFilter, deviceFilter, statusFilter],
  );

  const enrollmentsQuery = useEnrollmentsQuery(enabled, listFilter);
  const createEnrollment = useCreateEnrollment();
  const cancelEnrollment = useCancelEnrollment();
  const revokeEnrollment = useRevokeEnrollment();
  const retryEnrollment = useRetryEnrollment();

  const createForm = useForm<CreateForm>({
    resolver: zodResolver(createSchema),
    defaultValues: { userId: "", credentialId: "", deviceId: "" },
  });
  const selectedUserId = createForm.watch("userId");
  const credentialsForUser = useCredentialsQuery(enabled && Boolean(selectedUserId), {
    userId: selectedUserId || undefined,
    status: "active",
  });

  const filtered = useMemo(() => {
    const rows = enrollmentsQuery.data ?? [];
    const query = search.trim().toLowerCase();
    if (!query) {
      return rows;
    }
    return rows.filter((row) => {
      return (
        row.userName.toLowerCase().includes(query) ||
        row.deviceName.toLowerCase().includes(query) ||
        row.credentialType.toLowerCase().includes(query) ||
        row.status.toLowerCase().includes(query)
      );
    });
  }, [enrollmentsQuery.data, search]);

  async function onCreate(values: CreateForm) {
    try {
      await createEnrollment.mutateAsync(values);
      toast({ title: "Enrollment created", variant: "success" });
      createForm.reset({ userId: "", credentialId: "", deviceId: "" });
      setCreateOpen(false);
    } catch (reason: unknown) {
      toast({
        title: "Could not create enrollment",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onConfirmCancel() {
    if (!cancelTarget) {
      return;
    }
    try {
      await cancelEnrollment.mutateAsync(cancelTarget.id);
      toast({ title: "Enrollment cancelled", variant: "success" });
      setCancelTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not cancel enrollment",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onConfirmRevoke() {
    if (!revokeTarget) {
      return;
    }
    try {
      await revokeEnrollment.mutateAsync(revokeTarget.id);
      toast({ title: "Enrollment revoked", variant: "success" });
      setRevokeTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not revoke enrollment",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onRetry(enrollment: Enrollment) {
    try {
      await retryEnrollment.mutateAsync(enrollment.id);
      toast({ title: "Enrollment queued for retry", variant: "success" });
    } catch (reason: unknown) {
      toast({
        title: "Could not retry enrollment",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  if (!enabled) {
    return (
      <div>
        <PageHeader
          title="Enrollments"
          description="Assign user credentials to devices as desired state."
        />
        <ErrorState
          title="Database required"
          description="Connect local PostgreSQL from Dashboard before managing enrollments."
        />
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Enrollments"
        description="Desired assignments of credentials to devices. Synchronization to Matrix is a later slice."
        action={
          <Button type="button" onClick={() => setCreateOpen(true)}>
            <Plus className="h-4 w-4" aria-hidden />
            Add Enrollment
          </Button>
        }
      />

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <div className="relative min-w-[12rem] flex-1">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground"
            aria-hidden
          />
          <Input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search enrollments…"
            className="pl-8"
            aria-label="Search enrollments"
          />
        </div>
        <Select value={userFilter} onValueChange={setUserFilter}>
          <SelectTrigger className="w-40" aria-label="Filter by user">
            <SelectValue placeholder="User" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All users</SelectItem>
            {(usersQuery.data ?? []).map((user) => (
              <SelectItem key={user.id} value={user.id}>
                {user.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={deviceFilter} onValueChange={setDeviceFilter}>
          <SelectTrigger className="w-40" aria-label="Filter by device">
            <SelectValue placeholder="Device" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All devices</SelectItem>
            {(devicesQuery.data ?? []).map((device) => (
              <SelectItem key={device.id} value={device.id}>
                {device.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          value={statusFilter}
          onValueChange={(value: "all" | EnrollmentStatus) =>
            setStatusFilter(value)
          }
        >
          <SelectTrigger className="w-36" aria-label="Filter by status">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All statuses</SelectItem>
            <SelectItem value="pending">Pending</SelectItem>
            <SelectItem value="active">Active</SelectItem>
            <SelectItem value="failed">Failed</SelectItem>
            <SelectItem value="cancelled">Cancelled</SelectItem>
            <SelectItem value="revoked">Revoked</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {enrollmentsQuery.isLoading ? (
        <div className="rounded-lg border border-border bg-card px-4 py-6 text-sm text-muted-foreground">
          Loading enrollments…
        </div>
      ) : null}

      {enrollmentsQuery.isError ? (
        <ErrorState
          title="Unable to load enrollments"
          description={asErrorMessage(enrollmentsQuery.error)}
          onRetry={() => void enrollmentsQuery.refetch()}
        />
      ) : null}

      {!enrollmentsQuery.isLoading &&
      !enrollmentsQuery.isError &&
      (enrollmentsQuery.data?.length ?? 0) === 0 ? (
        <EmptyState
          title="No enrollments yet"
          description="Create an enrollment to assign a user credential to a device."
          actionLabel="Add Enrollment"
          onAction={() => setCreateOpen(true)}
        />
      ) : null}

      {!enrollmentsQuery.isLoading &&
      !enrollmentsQuery.isError &&
      (enrollmentsQuery.data?.length ?? 0) > 0 &&
      filtered.length === 0 ? (
        <EmptyState
          title="No matching enrollments"
          description="Try a different search or clear filters."
        />
      ) : null}

      {filtered.length > 0 ? (
        <div className="overflow-hidden rounded-lg border border-border bg-card">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">User</th>
                <th className="px-3 py-2 font-medium">Credential</th>
                <th className="px-3 py-2 font-medium">Device</th>
                <th className="px-3 py-2 font-medium">Status</th>
                <th className="px-3 py-2 font-medium">Updated</th>
                <th className="px-3 py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((enrollment) => (
                <tr key={enrollment.id} className="border-t border-border">
                  <td className="px-3 py-2 font-medium text-foreground">
                    {enrollment.userName}
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {credentialLabel(
                      enrollment.credentialType,
                      enrollment.maskedValue,
                    )}
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {enrollment.deviceName}
                  </td>
                  <td className="px-3 py-2">
                    <EnrollmentStatusBadge status={enrollment.status} />
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {formatDateTime(enrollment.updatedAt)}
                  </td>
                  <td className="px-3 py-2 text-right">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={`Actions for enrollment ${enrollment.userName}`}
                        >
                          <MoreHorizontal className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        {enrollment.status === "failed" ? (
                          <DropdownMenuItem
                            onSelect={() => void onRetry(enrollment)}
                          >
                            Retry
                          </DropdownMenuItem>
                        ) : null}
                        {enrollment.status === "pending" ||
                        enrollment.status === "failed" ? (
                          <DropdownMenuItem
                            onSelect={() => setCancelTarget(enrollment)}
                          >
                            Cancel
                          </DropdownMenuItem>
                        ) : null}
                        {enrollment.status === "active" ? (
                          <>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              destructive
                              onSelect={() => setRevokeTarget(enrollment)}
                            >
                              Revoke
                            </DropdownMenuItem>
                          </>
                        ) : null}
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
            <DialogTitle>Add enrollment</DialogTitle>
            <DialogDescription>
              Records that this credential should be associated with the device.
              Device online status is not required.
            </DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-3"
            onSubmit={createForm.handleSubmit(onCreate)}
          >
            <div className="grid gap-1.5">
              <Label>User</Label>
              <Select
                value={createForm.watch("userId") || undefined}
                onValueChange={(value) => {
                  createForm.setValue("userId", value, { shouldValidate: true });
                  createForm.setValue("credentialId", "");
                }}
              >
                <SelectTrigger aria-label="User">
                  <SelectValue placeholder="Select user" />
                </SelectTrigger>
                <SelectContent>
                  {(usersQuery.data ?? [])
                    .filter((user) => user.status === "active")
                    .map((user) => (
                      <SelectItem key={user.id} value={user.id}>
                        {user.name}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
              {createForm.formState.errors.userId ? (
                <p className="text-xs text-destructive">
                  {createForm.formState.errors.userId.message}
                </p>
              ) : null}
            </div>
            <div className="grid gap-1.5">
              <Label>Credential</Label>
              <Select
                value={createForm.watch("credentialId") || undefined}
                onValueChange={(value) =>
                  createForm.setValue("credentialId", value, {
                    shouldValidate: true,
                  })
                }
                disabled={!selectedUserId}
              >
                <SelectTrigger aria-label="Credential">
                  <SelectValue placeholder="Select credential" />
                </SelectTrigger>
                <SelectContent>
                  {(credentialsForUser.data ?? []).map((credential) => (
                    <SelectItem key={credential.id} value={credential.id}>
                      {credentialLabel(
                        credential.credentialType,
                        credential.maskedValue,
                      )}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {createForm.formState.errors.credentialId ? (
                <p className="text-xs text-destructive">
                  {createForm.formState.errors.credentialId.message}
                </p>
              ) : null}
            </div>
            <div className="grid gap-1.5">
              <Label>Device</Label>
              <Select
                value={createForm.watch("deviceId") || undefined}
                onValueChange={(value) =>
                  createForm.setValue("deviceId", value, { shouldValidate: true })
                }
              >
                <SelectTrigger aria-label="Device">
                  <SelectValue placeholder="Select device" />
                </SelectTrigger>
                <SelectContent>
                  {(devicesQuery.data ?? []).map((device) => (
                    <SelectItem key={device.id} value={device.id}>
                      {device.name} ({device.host}:{device.port})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {createForm.formState.errors.deviceId ? (
                <p className="text-xs text-destructive">
                  {createForm.formState.errors.deviceId.message}
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
              <Button type="submit" disabled={createEnrollment.isPending}>
                {createEnrollment.isPending ? "Saving…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(cancelTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setCancelTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Cancel enrollment?</DialogTitle>
            <DialogDescription>
              This marks the desired assignment as cancelled. The record is kept.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setCancelTarget(null)}
            >
              Keep
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={cancelEnrollment.isPending}
              onClick={() => void onConfirmCancel()}
            >
              {cancelEnrollment.isPending ? "Working…" : "Cancel enrollment"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(revokeTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setRevokeTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Revoke enrollment?</DialogTitle>
            <DialogDescription>
              This withdraws the active desired assignment. Device removal is
              handled by a future Sync slice.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setRevokeTarget(null)}
            >
              Keep
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={revokeEnrollment.isPending}
              onClick={() => void onConfirmRevoke()}
            >
              {revokeEnrollment.isPending ? "Working…" : "Revoke"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
