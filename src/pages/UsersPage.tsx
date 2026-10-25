import { useEffect, useMemo, useState, type FormEvent } from "react";
import { UserRound } from "lucide-react";
import { EmptyState, ErrorState } from "@/components/layout/PageHeader";
import { FilterBar, PageControls, ResultSummary, SearchField } from "@/components/lists/ListControls";
import { StatusBadge } from "@/components/StatusBadge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useToast } from "@/components/ui/toaster";
import { asErrorMessage, useDeleteUser, useRegisterUser, useUsersQuery } from "@/hooks/useUsers";
import { usePagedItems } from "@/lib/paging";
import type { User, UserStatus } from "@/types/users";

type UsersPageProps = {
  enabled: boolean;
  onEnroll: (userId: string) => void;
};

export function UsersPage({ enabled, onEnroll }: UsersPageProps) {
  const usersQuery = useUsersQuery(enabled);
  const deleteUser = useDeleteUser();
  const registerUser = useRegisterUser();
  const { toast } = useToast();
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all" | UserStatus>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [registering, setRegistering] = useState(false);

  const users = usersQuery.data ?? [];
  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return users.filter((user) => {
      if (statusFilter !== "all" && user.status !== statusFilter) {
        return false;
      }
      if (!query) {
        return true;
      }
      return (
        user.username.toLowerCase().includes(query) ||
        (user.fullName ?? "").toLowerCase().includes(query) ||
        (user.shortName ?? "").toLowerCase().includes(query) ||
        (user.matrixUserId ?? "").toLowerCase().includes(query) ||
        String(user.referenceId ?? "").includes(query)
      );
    });
  }, [search, statusFilter, users]);

  useEffect(() => {
    if (filtered.length === 0) {
      setSelectedId(null);
      return;
    }
    if (!filtered.some((user) => user.id === selectedId)) {
      setSelectedId(filtered[0]?.id ?? null);
    }
  }, [filtered, selectedId]);

  const selected = users.find((user) => user.id === selectedId) ?? null;

  async function onDelete(user: User) {
    try {
      await deleteUser.mutateAsync(user.id);
      toast({
        title: "User removed",
        description: `${user.username} was removed.`,
        variant: "success",
      });
    } catch (reason: unknown) {
      toast({
        title: "Could not remove user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onRegister(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    try {
      const user = await registerUser.mutateAsync({
        matrixUserId: String(form.get("matrixUserId") ?? ""),
        username: String(form.get("username") ?? ""),
        shortName: String(form.get("shortName") ?? ""),
        fullName: String(form.get("fullName") ?? ""),
        referenceId: String(form.get("referenceId") ?? ""),
        active: form.get("active") === "on",
      });
      setRegistering(false);
      setSelectedId(user.id);
      toast({
        title: "User registered",
        description: `${user.username} is saved.`,
        variant: "success",
      });
    } catch (reason: unknown) {
      toast({
        title: "Could not register user",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  if (!enabled) {
    return (
      <div>
        <p className="text-sm text-muted-foreground">Identity and status for each person.</p>
        <div className="mt-4">
          <ErrorState
            title="Database required"
            description="Connect local PostgreSQL from Dashboard before opening users."
          />
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center justify-between gap-3 border-b border-border bg-background pb-3">
        <div className="min-w-0">
          <p className="truncate text-sm text-muted-foreground">
            Register one person here, or add a list from Import User.
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {registering ? (
            <Button type="button" variant="secondary" onClick={() => setRegistering(false)}>
              Cancel
            </Button>
          ) : null}
          <Button
            type="submit"
            form="register-user-form"
            disabled={!registering || registerUser.isPending}
            className={registering ? undefined : "hidden"}
          >
            {registerUser.isPending ? "Saving…" : "Save"}
          </Button>
          <Button
            type="button"
            variant={registering ? "secondary" : "default"}
            aria-pressed={registering}
            onClick={() => setRegistering(true)}
          >
            Register User
          </Button>
          <Button
            type="button"
            variant="outline"
            disabled={!selected || registering}
            onClick={() => {
              if (selected) {
                onEnroll(selected.id);
              }
            }}
          >
            Enrollment
          </Button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto pt-4">
        {registering ? (
          <RegisterUserForm pending={registerUser.isPending} onSubmit={onRegister} />
        ) : (
          <UserList
            search={search}
            onSearch={setSearch}
            statusFilter={statusFilter}
            onStatusFilter={setStatusFilter}
            loading={usersQuery.isLoading}
            error={usersQuery.isError ? asErrorMessage(usersQuery.error) : null}
            onRetry={() => void usersQuery.refetch()}
            users={users}
            filtered={filtered}
            selected={selected}
            onSelect={setSelectedId}
            removing={deleteUser.isPending}
            onDelete={(user) => void onDelete(user)}
          />
        )}
      </div>
    </div>
  );
}

function RegisterUserForm({
  pending,
  onSubmit,
}: {
  pending: boolean;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}) {
  const [matrixUserId, setMatrixUserId] = useState("");
  const [username, setUsername] = useState("");
  const [active, setActive] = useState(true);

  return (
    <div className="grid gap-4 lg:grid-cols-[14rem_minmax(0,1fr)]">
      <aside className="h-fit rounded-lg border border-border bg-card px-4 py-5 text-center">
        <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-muted text-muted-foreground">
          <UserRound className="h-8 w-8" aria-hidden />
        </div>
        <p className="mt-3 text-sm font-medium text-foreground">{matrixUserId || "New user"}</p>
        <p className="text-xs text-muted-foreground">{username || "Name"}</p>
        <p className={`mt-1 text-xs ${active ? "text-emerald-700" : "text-muted-foreground"}`}>
          {active ? "Active" : "Inactive"}
        </p>
      </aside>
      <form
        id="register-user-form"
        className="rounded-lg border border-border bg-card"
        onSubmit={onSubmit}
      >
        <div className="border-b border-border bg-muted/40 px-4">
          <span className="inline-block border-b-2 border-primary px-3 py-2 text-sm font-medium text-foreground">
            Basic
          </span>
        </div>
        <div className="space-y-3 px-6 py-5">
          <FormRow
            id="register-id"
            label="ID"
            name="matrixUserId"
            value={matrixUserId}
            required
            disabled={pending}
            onChange={setMatrixUserId}
          />
          <FormRow
            id="register-name"
            label="Name"
            name="username"
            value={username}
            required
            disabled={pending}
            onChange={setUsername}
          />
          <div className="grid grid-cols-[9rem_auto] items-center gap-3">
            <label htmlFor="register-active" className="text-right text-sm text-foreground">
              Active
            </label>
            <input
              id="register-active"
              name="active"
              type="checkbox"
              checked={active}
              disabled={pending}
              onChange={(event) => setActive(event.target.checked)}
              className="h-4 w-4 rounded border border-border"
            />
          </div>
          <fieldset className="rounded-md border border-border px-4 py-4">
            <legend className="px-1 text-xs text-muted-foreground">Optional</legend>
            <div className="space-y-3">
              <FormRow id="register-full-name" label="Full Name" name="fullName" disabled={pending} />
              <FormRow
                id="register-short-name"
                label="Short Name"
                name="shortName"
                required
                disabled={pending}
              />
              <FormRow
                id="register-reference"
                label="Reference ID"
                name="referenceId"
                required
                disabled={pending}
              />
            </div>
          </fieldset>
        </div>
      </form>
    </div>
  );
}

function FormRow({
  id,
  label,
  name,
  value,
  required,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  name: string;
  value?: string;
  required?: boolean;
  disabled?: boolean;
  onChange?: (value: string) => void;
}) {
  return (
    <div className="grid grid-cols-[9rem_minmax(0,16rem)] items-center gap-3">
      <label htmlFor={id} className="text-right text-sm text-foreground">
        {label}
        {required ? <span className="text-destructive"> *</span> : null}
      </label>
      <Input
        id={id}
        name={name}
        value={value}
        required={required}
        disabled={disabled}
        onChange={onChange ? (event) => onChange(event.target.value) : undefined}
      />
    </div>
  );
}

function UserList({
  search,
  onSearch,
  statusFilter,
  onStatusFilter,
  loading,
  error,
  onRetry,
  users,
  filtered,
  selected,
  onSelect,
  removing,
  onDelete,
}: {
  search: string;
  onSearch: (value: string) => void;
  statusFilter: "all" | UserStatus;
  onStatusFilter: (value: "all" | UserStatus) => void;
  loading: boolean;
  error: string | null;
  onRetry: () => void;
  users: User[];
  filtered: User[];
  selected: User | null;
  onSelect: (id: string) => void;
  removing: boolean;
  onDelete: (user: User) => void;
}) {
  const page = usePagedItems(filtered, 50, `${search}|${statusFilter}`);
  return (
    <>
      <FilterBar>
        <SearchField
          value={search}
          onChange={onSearch}
          placeholder="Search name, full name, ID, or reference"
          label="Search users"
        />
        <StatusChoice value={statusFilter} onChange={onStatusFilter} />
      </FilterBar>
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <ResultSummary
          start={page.start}
          count={page.slice.length}
          total={page.total}
          noun="people"
        />
        <PageControls page={page.page} pageCount={page.pageCount} onPage={page.setPage} />
      </div>
      {loading ? <p className="text-sm text-muted-foreground">Loading users…</p> : null}
      {error ? (
        <ErrorState title="Unable to load users" description={error} onRetry={onRetry} />
      ) : null}
      {!loading && !error && users.length === 0 ? (
        <EmptyState
          title="No users yet"
          description="Use Register User at the top of this page, or import an Excel list."
        />
      ) : null}
      {!loading && !error && users.length > 0 && filtered.length === 0 ? (
        <EmptyState
          title="No matching people"
          description="Try another name, ID, or reference, or clear the status filter."
        />
      ) : null}
      {filtered.length > 0 ? (
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_20rem]">
          <div className="overflow-x-auto rounded-lg border border-border bg-card">
            <table className="w-full min-w-[36rem] border-collapse text-sm">
              <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
                <tr>
                  <th className="px-3 py-2 font-medium">Name</th>
                  <th className="px-3 py-2 font-medium">ID</th>
                  <th className="px-3 py-2 font-medium">Reference</th>
                  <th className="px-3 py-2 font-medium">Status</th>
                </tr>
              </thead>
              <tbody>
                {page.slice.map((user) => {
                  const active = user.id === selected?.id;
                  return (
                    <tr
                      key={user.id}
                      className={`cursor-pointer border-t border-border ${active ? "bg-accent" : "hover:bg-muted"}`}
                      onClick={() => onSelect(user.id)}
                    >
                      <td className="px-3 py-2">
                        <button
                          type="button"
                          className="text-left"
                          aria-current={active ? "true" : undefined}
                          onClick={() => onSelect(user.id)}
                        >
                          <span className="block font-medium text-foreground">{user.username}</span>
                          <span className="block text-xs text-muted-foreground">
                            {user.fullName ?? "Full name not reported"}
                          </span>
                        </button>
                      </td>
                      <td className="px-3 py-2 text-muted-foreground">
                        {user.matrixUserId ?? "Not imported"}
                      </td>
                      <td className="px-3 py-2 text-muted-foreground">
                        {user.referenceId == null ? "—" : String(user.referenceId)}
                      </td>
                      <td className="px-3 py-2">
                        <StatusBadge status={user.status} />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
          {selected ? (
            <section className="rounded-lg border border-border bg-card">
              <div className="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
                <div>
                  <h3 className="text-base font-semibold text-foreground">{selected.username}</h3>
                  <p className="text-xs text-muted-foreground">
                    {selected.matrixUserId ?? "Not imported"}
                  </p>
                </div>
                <StatusBadge status={selected.status} />
              </div>
              <dl className="grid gap-3 px-4 py-4 sm:grid-cols-2">
                <Detail label="ID" value={selected.matrixUserId ?? "Not reported"} />
                <Detail label="Name" value={selected.username} />
                <Detail label="Active" value={selected.status === "active" ? "Yes" : "No"} />
                <Detail label="Full Name" value={selected.fullName ?? "Not reported"} />
                <Detail label="Short Name" value={selected.shortName ?? "Not reported"} />
                <Detail
                  label="Reference ID"
                  value={selected.referenceId == null ? "Not reported" : String(selected.referenceId)}
                />
              </dl>
              <div className="border-t border-border px-4 py-3">
                <Button
                  type="button"
                  variant="secondary"
                  disabled={removing}
                  onClick={() => onDelete(selected)}
                >
                  {removing ? "Removing…" : "Remove user"}
                </Button>
              </div>
            </section>
          ) : null}
        </div>
      ) : null}
    </>
  );
}

function StatusChoice({
  value,
  onChange,
}: {
  value: "all" | UserStatus;
  onChange: (value: "all" | UserStatus) => void;
}) {
  const options: Array<{ id: "all" | UserStatus; label: string }> = [
    { id: "all", label: "All" },
    { id: "active", label: "Active" },
    { id: "inactive", label: "Inactive" },
  ];
  return (
    <div className="flex rounded-md border border-border bg-card p-0.5" role="group" aria-label="Status">
      {options.map((option) => (
        <button
          key={option.id}
          type="button"
          aria-pressed={value === option.id}
          className={`rounded px-2.5 py-1 text-xs ${
            value === option.id ? "bg-primary text-primary-foreground" : "text-muted-foreground"
          }`}
          onClick={() => onChange(option.id)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-muted-foreground">{label}</dt>
      <dd className="mt-1 text-sm text-foreground">{value}</dd>
    </div>
  );
}
