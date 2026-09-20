import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { MoreHorizontal, Plus, Search } from "lucide-react";
import { PageHeader, EmptyState, ErrorState } from "@/components/layout/PageHeader";
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
import {
  asErrorMessage,
  useCreateCredential,
  useCredentialsQuery,
  useSetCredentialStatus,
  useUpdateCredential,
} from "@/hooks/useCredentials";
import { useUsersQuery } from "@/hooks/useUsers";
import { formatDateTime } from "@/lib/utils";
import type {
  Credential,
  CredentialStatus,
  CredentialType,
} from "@/types/credentials";

const createSchema = z
  .object({
    userId: z.string().uuid("Select a user."),
    credentialType: z.enum(["card", "pin"]),
    value: z.string().min(1, "Value is required."),
  })
  .superRefine((data, ctx) => {
    if (data.credentialType === "pin") {
      if (!/^\d{4,12}$/.test(data.value.trim())) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["value"],
          message: "PIN must be 4–12 digits.",
        });
      }
    } else if (data.value.trim().length === 0 || data.value.trim().length > 64) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["value"],
        message: "Enter a card number (max 64 characters).",
      });
    }
  });

const updateValueSchema = z.object({
  value: z.string().min(1, "Enter a new value, or cancel."),
});

type CreateForm = z.infer<typeof createSchema>;
type UpdateValueForm = z.infer<typeof updateValueSchema>;

type CredentialsPageProps = {
  enabled: boolean;
};

function credentialDisplay(credential: Credential): string {
  if (credential.credentialType === "pin") {
    return "Configured";
  }
  return credential.maskedValue ?? "Configured";
}

export function CredentialsPage({ enabled }: CredentialsPageProps) {
  const { toast } = useToast();
  const usersQuery = useUsersQuery(enabled);
  const [search, setSearch] = useState("");
  const [userFilter, setUserFilter] = useState<string>("all");
  const [typeFilter, setTypeFilter] = useState<"all" | CredentialType>("all");
  const [statusFilter, setStatusFilter] = useState<"all" | CredentialStatus>(
    "all",
  );
  const [createOpen, setCreateOpen] = useState(false);
  const [updateTarget, setUpdateTarget] = useState<Credential | null>(null);
  const [deactivateTarget, setDeactivateTarget] = useState<Credential | null>(
    null,
  );

  const listFilter = useMemo(
    () => ({
      userId: userFilter === "all" ? undefined : userFilter,
      credentialType: typeFilter === "all" ? undefined : typeFilter,
      status: statusFilter === "all" ? undefined : statusFilter,
    }),
    [userFilter, typeFilter, statusFilter],
  );

  const credentialsQuery = useCredentialsQuery(enabled, listFilter);
  const createCredential = useCreateCredential();
  const updateCredential = useUpdateCredential();
  const setStatus = useSetCredentialStatus();

  const createForm = useForm<CreateForm>({
    resolver: zodResolver(createSchema),
    defaultValues: {
      userId: "",
      credentialType: "card",
      value: "",
    },
  });
  const updateForm = useForm<UpdateValueForm>({
    resolver: zodResolver(updateValueSchema),
    defaultValues: { value: "" },
  });

  const selectedType = createForm.watch("credentialType");

  const filtered = useMemo(() => {
    const rows = credentialsQuery.data ?? [];
    const query = search.trim().toLowerCase();
    if (!query) {
      return rows;
    }
    return rows.filter((credential) => {
      return (
        credential.userName.toLowerCase().includes(query) ||
        credential.credentialType.toLowerCase().includes(query) ||
        (credential.maskedValue?.toLowerCase().includes(query) ?? false)
      );
    });
  }, [credentialsQuery.data, search]);

  async function onCreate(values: CreateForm) {
    try {
      await createCredential.mutateAsync({
        userId: values.userId,
        credentialType: values.credentialType,
        value: values.value,
      });
      toast({ title: "Credential created", variant: "success" });
      createForm.reset({
        userId: values.userId,
        credentialType: "card",
        value: "",
      });
      setCreateOpen(false);
    } catch (reason: unknown) {
      toast({
        title: "Could not create credential",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onUpdateValue(values: UpdateValueForm) {
    if (!updateTarget) {
      return;
    }
    try {
      await updateCredential.mutateAsync({
        id: updateTarget.id,
        value: values.value,
      });
      toast({ title: "Credential value updated", variant: "success" });
      updateForm.reset({ value: "" });
      setUpdateTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not update credential",
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
      await setStatus.mutateAsync({
        id: deactivateTarget.id,
        status: "inactive",
      });
      toast({
        title: "Credential deactivated",
        description: `${deactivateTarget.userName} · ${deactivateTarget.credentialType}`,
        variant: "success",
      });
      setDeactivateTarget(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not deactivate credential",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onActivate(credential: Credential) {
    try {
      await setStatus.mutateAsync({ id: credential.id, status: "active" });
      toast({ title: "Credential activated", variant: "success" });
    } catch (reason: unknown) {
      toast({
        title: "Could not activate credential",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  if (!enabled) {
    return (
      <div>
        <PageHeader
          title="Credentials"
          description="Manage user credentials in the local system of record."
        />
        <ErrorState
          title="Database required"
          description="Connect local PostgreSQL from Dashboard before managing credentials."
        />
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Credentials"
        description="Associate Card and PIN credentials with users. Secrets stay in Rust."
        action={
          <Button type="button" onClick={() => setCreateOpen(true)}>
            <Plus className="h-4 w-4" aria-hidden />
            Add Credential
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
            placeholder="Search by user or type…"
            className="pl-8"
            aria-label="Search credentials"
          />
        </div>
        <Select value={userFilter} onValueChange={setUserFilter}>
          <SelectTrigger className="w-44" aria-label="Filter by user">
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
        <Select
          value={typeFilter}
          onValueChange={(value: "all" | CredentialType) => setTypeFilter(value)}
        >
          <SelectTrigger className="w-32" aria-label="Filter by type">
            <SelectValue placeholder="Type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All types</SelectItem>
            <SelectItem value="card">Card</SelectItem>
            <SelectItem value="pin">PIN</SelectItem>
          </SelectContent>
        </Select>
        <Select
          value={statusFilter}
          onValueChange={(value: "all" | CredentialStatus) =>
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

      {credentialsQuery.isLoading ? (
        <div className="rounded-lg border border-border bg-card px-4 py-6 text-sm text-muted-foreground">
          Loading credentials…
        </div>
      ) : null}

      {credentialsQuery.isError ? (
        <ErrorState
          title="Unable to load credentials"
          description={asErrorMessage(credentialsQuery.error)}
          onRetry={() => void credentialsQuery.refetch()}
        />
      ) : null}

      {!credentialsQuery.isLoading &&
      !credentialsQuery.isError &&
      (credentialsQuery.data?.length ?? 0) === 0 ? (
        <EmptyState
          title="No credentials yet"
          description="Create a Card or PIN credential for a user to get started."
          actionLabel="Add Credential"
          onAction={() => setCreateOpen(true)}
        />
      ) : null}

      {!credentialsQuery.isLoading &&
      !credentialsQuery.isError &&
      (credentialsQuery.data?.length ?? 0) > 0 &&
      filtered.length === 0 ? (
        <EmptyState
          title="No matching credentials"
          description="Try a different search or clear filters."
        />
      ) : null}

      {filtered.length > 0 ? (
        <div className="overflow-hidden rounded-lg border border-border bg-card">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">User</th>
                <th className="px-3 py-2 font-medium">Type</th>
                <th className="px-3 py-2 font-medium">Credential</th>
                <th className="px-3 py-2 font-medium">Status</th>
                <th className="px-3 py-2 font-medium">Updated</th>
                <th className="px-3 py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((credential) => (
                <tr key={credential.id} className="border-t border-border">
                  <td className="px-3 py-2 font-medium text-foreground">
                    {credential.userName}
                  </td>
                  <td className="px-3 py-2 capitalize text-muted-foreground">
                    {credential.credentialType}
                  </td>
                  <td className="px-3 py-2 font-mono text-xs text-muted-foreground">
                    {credentialDisplay(credential)}
                  </td>
                  <td className="px-3 py-2">
                    <StatusBadge status={credential.status} />
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {formatDateTime(credential.updatedAt)}
                  </td>
                  <td className="px-3 py-2 text-right">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={`Actions for ${credential.userName} ${credential.credentialType}`}
                        >
                          <MoreHorizontal className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onSelect={() => {
                            setUpdateTarget(credential);
                            updateForm.reset({ value: "" });
                          }}
                        >
                          Replace value
                        </DropdownMenuItem>
                        {credential.status === "inactive" ? (
                          <DropdownMenuItem
                            onSelect={() => void onActivate(credential)}
                          >
                            Activate
                          </DropdownMenuItem>
                        ) : (
                          <>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                              destructive
                              onSelect={() => setDeactivateTarget(credential)}
                            >
                              Deactivate
                            </DropdownMenuItem>
                          </>
                        )}
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
            <DialogTitle>Add credential</DialogTitle>
            <DialogDescription>
              Stores the credential locally. The value is write-only and never
              shown again after save.
            </DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-3"
            onSubmit={createForm.handleSubmit(onCreate)}
          >
            <div className="grid gap-1.5">
              <Label htmlFor="credential-user">User</Label>
              <Select
                value={createForm.watch("userId") || undefined}
                onValueChange={(value) =>
                  createForm.setValue("userId", value, { shouldValidate: true })
                }
              >
                <SelectTrigger id="credential-user" aria-label="User">
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
              <Label htmlFor="credential-type">Type</Label>
              <Select
                value={selectedType}
                onValueChange={(value: CredentialType) => {
                  createForm.setValue("credentialType", value, {
                    shouldValidate: true,
                  });
                  createForm.setValue("value", "");
                }}
              >
                <SelectTrigger id="credential-type" aria-label="Credential type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="card">Card</SelectItem>
                  <SelectItem value="pin">PIN</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="credential-value">
                {selectedType === "pin" ? "PIN" : "Card number"}
              </Label>
              <Input
                id="credential-value"
                type={selectedType === "pin" ? "password" : "text"}
                autoComplete="off"
                maxLength={selectedType === "pin" ? 12 : 64}
                {...createForm.register("value")}
              />
              {createForm.formState.errors.value ? (
                <p className="text-xs text-destructive">
                  {createForm.formState.errors.value.message}
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
              <Button type="submit" disabled={createCredential.isPending}>
                {createCredential.isPending ? "Saving…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(updateTarget)}
        onOpenChange={(open) => {
          if (!open) {
            updateForm.reset({ value: "" });
            setUpdateTarget(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Replace credential value</DialogTitle>
            <DialogDescription>
              Write-only. The current value is never displayed. Leave blank is
              not allowed — cancel to keep the existing value.
            </DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-3"
            onSubmit={updateForm.handleSubmit(onUpdateValue)}
          >
            <div className="grid gap-1.5">
              <Label htmlFor="replace-value">
                {updateTarget?.credentialType === "pin"
                  ? "New PIN"
                  : "New card number"}
              </Label>
              <Input
                id="replace-value"
                type={
                  updateTarget?.credentialType === "pin" ? "password" : "text"
                }
                autoComplete="off"
                maxLength={updateTarget?.credentialType === "pin" ? 12 : 64}
                {...updateForm.register("value")}
              />
              {updateForm.formState.errors.value ? (
                <p className="text-xs text-destructive">
                  {updateForm.formState.errors.value.message}
                </p>
              ) : null}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setUpdateTarget(null)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={updateCredential.isPending}>
                {updateCredential.isPending ? "Saving…" : "Replace"}
              </Button>
            </DialogFooter>
          </form>
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
            <DialogTitle>Deactivate credential?</DialogTitle>
            <DialogDescription>
              This marks the credential inactive for future operations. The
              record is kept; it is not deleted.
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
              disabled={setStatus.isPending}
              onClick={() => void onConfirmDeactivate()}
            >
              {setStatus.isPending ? "Working…" : "Deactivate"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
