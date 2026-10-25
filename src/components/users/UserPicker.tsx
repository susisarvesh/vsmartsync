import { useMemo, useState } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { User } from "@/types/users";

export function UserPicker({
  id,
  users,
  value,
  onChange,
  disabled,
}: {
  id: string;
  users: User[];
  value: string;
  onChange: (userId: string) => void;
  disabled?: boolean;
}) {
  const [query, setQuery] = useState("");
  const selected = users.find((user) => user.id === value) ?? null;
  const matches = useMemo(() => {
    const text = query.trim().toLowerCase();
    if (text.length < 1) {
      return [];
    }
    return users
      .filter((user) => user.status === "active")
      .filter((user) => matchesUser(user, text))
      .slice(0, 8);
  }, [query, users]);

  return (
    <div className="grid gap-1.5">
      <Label htmlFor={id}>User</Label>
      {selected ? (
        <div className="flex items-center justify-between gap-2 rounded-md border border-border bg-muted px-2 py-1.5 text-sm">
          <span className="min-w-0 truncate">
            {selected.username}
            <span className="ml-2 text-xs text-muted-foreground">
              {selected.matrixUserId ?? "No ID"}
            </span>
          </span>
          <button
            type="button"
            className="text-xs text-primary"
            disabled={disabled}
            onClick={() => {
              onChange("");
              setQuery("");
            }}
          >
            Change
          </button>
        </div>
      ) : (
        <>
          <Input
            id={id}
            value={query}
            disabled={disabled}
            placeholder="Type a name, ID, or reference"
            aria-label="Find a user"
            autoComplete="off"
            onChange={(event) => setQuery(event.target.value)}
          />
          {query.trim().length > 0 ? (
            <ul className="overflow-hidden rounded-md border border-border bg-card">
              {matches.length === 0 ? (
                <li className="px-3 py-2 text-sm text-muted-foreground">No matching people</li>
              ) : (
                matches.map((user) => (
                  <li key={user.id} className="border-b border-border last:border-b-0">
                    <button
                      type="button"
                      className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-muted"
                      onClick={() => {
                        onChange(user.id);
                        setQuery("");
                      }}
                    >
                      <span className="font-medium">{user.username}</span>
                      <span className="text-xs text-muted-foreground">
                        {user.matrixUserId ?? "No ID"}
                      </span>
                    </button>
                  </li>
                ))
              )}
            </ul>
          ) : (
            <p className="text-xs text-muted-foreground">
              Search the directory. The full list is not shown here.
            </p>
          )}
        </>
      )}
    </div>
  );
}

function matchesUser(user: User, query: string): boolean {
  return (
    user.username.toLowerCase().includes(query) ||
    (user.fullName ?? "").toLowerCase().includes(query) ||
    (user.shortName ?? "").toLowerCase().includes(query) ||
    (user.matrixUserId ?? "").toLowerCase().includes(query) ||
    String(user.referenceId ?? "").includes(query)
  );
}
