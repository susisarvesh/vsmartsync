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
  useCreateUser,
  useDeactivateUser,
  useUpdateUser,
  useUsersQuery,
} from "@/hooks/useUsers";
import { formatDateTime } from "@/lib/utils";
import type { User } from "@/types/users";

const nameSchema = z.object({
  name: z
    .string()
    .trim()
    .min(1, "Name must contain at least 1 character.")
    .max(200, "Name must be 200 characters or fewer."),
});

type NameForm = z.infer<typeof nameSchema>;

type UsersPageProps = {
  enabled: boolean;
};

export function UsersPage({ enabled }: UsersPageProps) {
  const { toast } = useToast();
  const usersQuery = useUsersQuery(enabled);
  const createUser = useCreateUser();
  const updateUser = useUpdateUser();
  const deactivateUser = useDeactivateUser();

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all" | "active" | "inactive">(
    "all",
  );
  const [createOpen, setCreateOpen] = useState(false);
  const [renameUser, setRenameUser] = useState<User | null>(null);
  const [deactivateTarget, setDeactivateTarget] = useState<User | null>(null);

  const createForm = useForm<NameForm>({
    resolver: zodResolver(nameSchema),
    defaultValues: { name: "" },
  });
  const renameForm = useForm<NameForm>({
    resolver: zodResolver(nameSchema),
    defaultValues: { name: "" },
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
      return user.name.toLowerCase().includes(query);
    });
  }, [usersQuery.data, search, statusFilter]);

  async function onCreate(values: NameForm) {
    try {
      await createUser.mutateAsync(values.name);
      toast({
        title: "User created successfully",
        variant: "success",
      });
      createForm.reset({ name: "" });
      setCreateOpen(false);
    } catch (reason: unknown) {
      toast({
        title: "Could not create user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onRename(values: NameForm) {
    if (!renameUser) {
      return;
    }
    try {
      await updateUser.mutateAsync({ id: renameUser.id, name: values.name });
      toast({
        title: "User updated successfully",
        variant: "success",
      });
      setRenameUser(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not rename user",
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
        description: `${deactivateTarget.name} is now inactive.`,
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
        description="Manage users registered in the system. Status changes only through Deactivate."
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
        <div className="overflow-hidden rounded-lg border border-border bg-card">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">Name</th>
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
                    {user.name}
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
                          aria-label={`Actions for ${user.name}`}
                        >
                          <MoreHorizontal className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onSelect={() => {
                            setRenameUser(user);
                            renameForm.reset({ name: user.name });
                          }}
                        >
                          Rename
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          destructive
                          disabled={user.status === "inactive"}
                          onSelect={() => setDeactivateTarget(user)}
                        >
                          Deactivate
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
              <Label htmlFor="create-user-name">Name</Label>
              <Input
                id="create-user-name"
                autoComplete="off"
                maxLength={200}
                {...createForm.register("name")}
              />
              {createForm.formState.errors.name ? (
                <p className="text-xs text-destructive">
                  {createForm.formState.errors.name.message}
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
        open={Boolean(renameUser)}
        onOpenChange={(open) => {
          if (!open) {
            setRenameUser(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename user</DialogTitle>
            <DialogDescription>
              Changes the display name only. Status is not edited here.
            </DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-3"
            onSubmit={renameForm.handleSubmit(onRename)}
          >
            <div className="grid gap-1.5">
              <Label htmlFor="rename-user-name">Name</Label>
              <Input
                id="rename-user-name"
                autoComplete="off"
                maxLength={200}
                {...renameForm.register("name")}
              />
              {renameForm.formState.errors.name ? (
                <p className="text-xs text-destructive">
                  {renameForm.formState.errors.name.message}
                </p>
              ) : null}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setRenameUser(null)}
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
              This marks{" "}
              <strong>{deactivateTarget?.name}</strong> as inactive in the local
              system of record. The record is kept; it is not deleted. You can
              rename inactive users later.
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
    </div>
  );
}
