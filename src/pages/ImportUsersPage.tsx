import { useState } from "react";
import { EmptyState, ErrorState, PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toaster";
import { asErrorMessage, useImportUsers } from "@/hooks/useUsers";
import type { ImportRowError } from "@/types/users";

type ImportUsersPageProps = {
  enabled: boolean;
};

export function ImportUsersPage({ enabled }: ImportUsersPageProps) {
  const importUsers = useImportUsers();
  const { toast } = useToast();
  const [fileName, setFileName] = useState("");
  const [errors, setErrors] = useState<ImportRowError[]>([]);

  async function onFile(file: File | undefined) {
    if (!file) {
      return;
    }
    const lower = file.name.toLowerCase();
    if (!lower.endsWith(".xls") && !lower.endsWith(".xlsx")) {
      toast({
        title: "Use an Excel file",
        description: "The list must be .xls or .xlsx.",
        variant: "destructive",
      });
      return;
    }
    setFileName(file.name);
    setErrors([]);
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    try {
      const result = await importUsers.mutateAsync(bytes);
      if (result.errors.length > 0) {
        setErrors(result.errors);
        toast({
          title: "Nothing was imported",
          description: "Fix the listed rows and import the file again.",
          variant: "destructive",
        });
        return;
      }
      toast({
        title: "Users imported",
        description: `${result.imported} ${result.imported === 1 ? "person" : "people"} saved. Enroll them from User Configuration.`,
        variant: "success",
      });
    } catch (reason: unknown) {
      toast({
        title: "Could not import users",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  if (!enabled) {
    return (
      <div>
        <PageHeader
          title="Import User"
          description="Add people from an Excel list. ID, Name, Short Name, and Reference ID are required."
        />
        <ErrorState
          title="Database required"
          description="Connect local PostgreSQL from Dashboard before importing users."
        />
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Import User"
        description="Choose the Excel list. ID, Name, Short Name, and Reference ID are required on every row. Full Name and Active are saved when the sheet includes them. A blank Active cell means the person is active."
        action={
          <Button type="button" disabled={importUsers.isPending} asChild>
            <label>
              {importUsers.isPending ? "Importing…" : "Choose Excel file"}
              <input
                type="file"
                accept=".xls,.xlsx,application/vnd.ms-excel,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                className="sr-only"
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  event.target.value = "";
                  void onFile(file);
                }}
              />
            </label>
          </Button>
        }
      />
      {fileName ? (
        <p className="mb-3 text-sm text-muted-foreground">Last file: {fileName}</p>
      ) : (
        <EmptyState
          title="No file chosen"
          description="The first row must name the columns. One invalid row rejects the whole file."
        />
      )}
      {errors.length > 0 ? (
        <ul className="rounded-lg border border-border bg-card text-sm">
          {errors.map((error) => (
            <li key={`${error.row}-${error.message}`} className="border-b border-border px-3 py-2 last:border-b-0">
              <span className="font-medium text-foreground">Row {error.row}.</span>{" "}
              <span className="text-muted-foreground">{error.message}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
