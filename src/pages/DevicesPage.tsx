import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { MoreHorizontal, Plus, Search } from "lucide-react";
import { PageHeader, EmptyState, ErrorState } from "@/components/layout/PageHeader";
import { ConnectionStatusBadge } from "@/components/StatusBadge";
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
import { useToast } from "@/components/ui/toaster";
import {
  asErrorMessage,
  useCreateDevice,
  useDevicesQuery,
  useSetDevicePassword,
  useTestDeviceConnection,
  useUpdateDevice,
} from "@/hooks/useDevices";
import { formatDateTime } from "@/lib/utils";
import type { Device } from "@/types/devices";

const deviceSchema = z.object({
  name: z
    .string()
    .trim()
    .min(1, "Name must contain at least 1 character.")
    .max(200, "Name must be 200 characters or fewer."),
  host: z
    .string()
    .trim()
    .min(1, "Host is required.")
    .max(255, "Host must be 255 characters or fewer.")
    .refine((value) => !/[/:?#@\\\s]/.test(value), {
      message: "Enter a hostname or IPv4 address only (no URL).",
    }),
  port: z.coerce
    .number()
    .int("Port must be a whole number.")
    .min(1, "Port must be at least 1.")
    .max(65535, "Port must be at most 65535."),
  username: z
    .string()
    .trim()
    .min(1, "Username is required.")
    .max(100, "Username must be 100 characters or fewer."),
});

const createSchema = deviceSchema.extend({
  password: z
    .string()
    .min(1, "Password is required.")
    .max(200, "Password must be 200 characters or fewer."),
});

const passwordSchema = z.object({
  password: z
    .string()
    .min(1, "Password is required.")
    .max(200, "Password must be 200 characters or fewer."),
});

type DeviceForm = z.infer<typeof deviceSchema>;
type CreateForm = z.infer<typeof createSchema>;
type PasswordForm = z.infer<typeof passwordSchema>;

type DevicesPageProps = {
  enabled: boolean;
};

export function DevicesPage({ enabled }: DevicesPageProps) {
  const { toast } = useToast();
  const devicesQuery = useDevicesQuery(enabled);
  const createDevice = useCreateDevice();
  const updateDevice = useUpdateDevice();
  const setPassword = useSetDevicePassword();
  const testConnection = useTestDeviceConnection();

  const [search, setSearch] = useState("");
  const [createOpen, setCreateOpen] = useState(false);
  const [editDevice, setEditDevice] = useState<Device | null>(null);
  const [passwordDevice, setPasswordDevice] = useState<Device | null>(null);
  const [testingId, setTestingId] = useState<string | null>(null);

  const createForm = useForm<CreateForm>({
    resolver: zodResolver(createSchema),
    defaultValues: {
      name: "",
      host: "",
      port: 80,
      username: "admin",
      password: "",
    },
  });
  const editForm = useForm<DeviceForm>({
    resolver: zodResolver(deviceSchema),
    defaultValues: { name: "", host: "", port: 80, username: "" },
  });
  const passwordForm = useForm<PasswordForm>({
    resolver: zodResolver(passwordSchema),
    defaultValues: { password: "" },
  });

  const filtered = useMemo(() => {
    const devices = devicesQuery.data ?? [];
    const query = search.trim().toLowerCase();
    if (!query) {
      return devices;
    }
    return devices.filter((device) => {
      const endpoint = `${device.host}:${device.port}`.toLowerCase();
      return (
        device.name.toLowerCase().includes(query) ||
        endpoint.includes(query) ||
        device.username.toLowerCase().includes(query)
      );
    });
  }, [devicesQuery.data, search]);

  async function onCreate(values: CreateForm) {
    try {
      await createDevice.mutateAsync(values);
      toast({
        title: "Device registered",
        variant: "success",
      });
      createForm.reset({
        name: "",
        host: "",
        port: 80,
        username: "admin",
        password: "",
      });
      setCreateOpen(false);
    } catch (reason: unknown) {
      toast({
        title: "Could not register device",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onEdit(values: DeviceForm) {
    if (!editDevice) {
      return;
    }
    try {
      await updateDevice.mutateAsync({ id: editDevice.id, ...values });
      toast({
        title: "Device updated",
        variant: "success",
      });
      setEditDevice(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not update device",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onSetPassword(values: PasswordForm) {
    if (!passwordDevice) {
      return;
    }
    try {
      await setPassword.mutateAsync({
        id: passwordDevice.id,
        password: values.password,
      });
      toast({
        title: "Device password updated",
        variant: "success",
      });
      passwordForm.reset({ password: "" });
      setPasswordDevice(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not update password",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onTestConnection(device: Device) {
    setTestingId(device.id);
    try {
      await testConnection.mutateAsync(device.id);
      toast({
        title: "Connection successful",
        description: `${device.name} is reachable.`,
        variant: "success",
      });
    } catch (reason: unknown) {
      toast({
        title: "Connection failed",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    } finally {
      setTestingId(null);
    }
  }

  if (!enabled) {
    return (
      <div>
        <PageHeader
          title="Devices"
          description="Register Matrix COSEC devices and verify reachability."
        />
        <ErrorState
          title="Database required"
          description="Connect local PostgreSQL from Dashboard before managing devices."
        />
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Devices"
        description="Register Matrix COSEC devices and verify that this application can reach them."
        action={
          <Button type="button" onClick={() => setCreateOpen(true)}>
            <Plus className="h-4 w-4" aria-hidden />
            Add Device
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
            placeholder="Search devices…"
            className="pl-8"
            aria-label="Search devices"
          />
        </div>
      </div>

      {devicesQuery.isLoading ? (
        <div className="rounded-lg border border-border bg-card px-4 py-6 text-sm text-muted-foreground">
          Loading devices…
        </div>
      ) : null}

      {devicesQuery.isError ? (
        <ErrorState
          title="Unable to load devices"
          description={asErrorMessage(devicesQuery.error)}
          onRetry={() => void devicesQuery.refetch()}
        />
      ) : null}

      {!devicesQuery.isLoading &&
      !devicesQuery.isError &&
      (devicesQuery.data?.length ?? 0) === 0 ? (
        <EmptyState
          title="No devices yet"
          description="Register a Matrix COSEC device to store connection details and test reachability."
          actionLabel="Add Device"
          onAction={() => setCreateOpen(true)}
        />
      ) : null}

      {!devicesQuery.isLoading &&
      !devicesQuery.isError &&
      (devicesQuery.data?.length ?? 0) > 0 &&
      filtered.length === 0 ? (
        <EmptyState
          title="No matching devices"
          description="Try a different search."
        />
      ) : null}

      {filtered.length > 0 ? (
        <div className="overflow-hidden rounded-lg border border-border bg-card">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-muted/60 text-left text-xs uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">Name</th>
                <th className="px-3 py-2 font-medium">Host:Port</th>
                <th className="px-3 py-2 font-medium">Connection Status</th>
                <th className="px-3 py-2 font-medium">Last Seen</th>
                <th className="px-3 py-2 font-medium">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((device) => (
                <tr key={device.id} className="border-t border-border">
                  <td className="px-3 py-2 font-medium text-foreground">
                    {device.name}
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {device.host}:{device.port}
                  </td>
                  <td className="px-3 py-2">
                    <ConnectionStatusBadge status={device.connectionStatus} />
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {device.lastSeenAt
                      ? formatDateTime(device.lastSeenAt)
                      : "Never"}
                  </td>
                  <td className="px-3 py-2 text-right">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={`Actions for ${device.name}`}
                        >
                          <MoreHorizontal className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          disabled={testingId === device.id}
                          onSelect={() => void onTestConnection(device)}
                        >
                          {testingId === device.id
                            ? "Testing…"
                            : "Test Connection"}
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onSelect={() => {
                            setEditDevice(device);
                            editForm.reset({
                              name: device.name,
                              host: device.host,
                              port: device.port,
                              username: device.username,
                            });
                          }}
                        >
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          onSelect={() => {
                            setPasswordDevice(device);
                            passwordForm.reset({ password: "" });
                          }}
                        >
                          Set Password
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
            <DialogTitle>Add device</DialogTitle>
            <DialogDescription>
              Stores connection details locally. The password is encrypted in
              Rust and never shown again.
            </DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-3"
            onSubmit={createForm.handleSubmit(onCreate)}
          >
            <DeviceFields
              form={createForm as never}
              idPrefix="create"
              includePassword
            />
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setCreateOpen(false)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={createDevice.isPending}>
                {createDevice.isPending ? "Saving…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(editDevice)}
        onOpenChange={(open) => {
          if (!open) {
            setEditDevice(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit device</DialogTitle>
            <DialogDescription>
              Updates name and connection target. Password is unchanged unless
              you use Set Password.
            </DialogDescription>
          </DialogHeader>
          <form className="grid gap-3" onSubmit={editForm.handleSubmit(onEdit)}>
            <DeviceFields form={editForm as never} idPrefix="edit" />
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setEditDevice(null)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={updateDevice.isPending}>
                {updateDevice.isPending ? "Saving…" : "Save"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(passwordDevice)}
        onOpenChange={(open) => {
          if (!open) {
            passwordForm.reset({ password: "" });
            setPasswordDevice(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Set device password</DialogTitle>
            <DialogDescription>
              Write-only. The current password is never displayed or returned
              to this window.
            </DialogDescription>
          </DialogHeader>
          <form
            className="grid gap-3"
            onSubmit={passwordForm.handleSubmit(onSetPassword)}
          >
            <div className="grid gap-1.5">
              <Label htmlFor="device-password">New password</Label>
              <Input
                id="device-password"
                type="password"
                autoComplete="new-password"
                maxLength={200}
                {...passwordForm.register("password")}
              />
              {passwordForm.formState.errors.password ? (
                <p className="text-xs text-destructive">
                  {passwordForm.formState.errors.password.message}
                </p>
              ) : null}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setPasswordDevice(null)}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={setPassword.isPending}>
                {setPassword.isPending ? "Saving…" : "Update password"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function DeviceFields({
  form,
  idPrefix,
  includePassword = false,
}: {
  form: {
    register: (name: string) => object;
    formState: { errors: Record<string, { message?: string } | undefined> };
  };
  idPrefix: string;
  includePassword?: boolean;
}) {
  const errors = form.formState.errors;

  return (
    <>
      <div className="grid gap-1.5">
        <Label htmlFor={`${idPrefix}-name`}>Name</Label>
        <Input
          id={`${idPrefix}-name`}
          autoComplete="off"
          maxLength={200}
          {...form.register("name")}
        />
        {errors.name ? (
          <p className="text-xs text-destructive">{errors.name.message}</p>
        ) : null}
      </div>
      <div className="grid grid-cols-3 gap-3">
        <div className="col-span-2 grid gap-1.5">
          <Label htmlFor={`${idPrefix}-host`}>Host</Label>
          <Input
            id={`${idPrefix}-host`}
            autoComplete="off"
            placeholder="192.168.1.10"
            maxLength={255}
            {...form.register("host")}
          />
          {errors.host ? (
            <p className="text-xs text-destructive">{errors.host.message}</p>
          ) : null}
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${idPrefix}-port`}>Port</Label>
          <Input
            id={`${idPrefix}-port`}
            type="number"
            min={1}
            max={65535}
            {...form.register("port")}
          />
          {errors.port ? (
            <p className="text-xs text-destructive">{errors.port.message}</p>
          ) : null}
        </div>
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor={`${idPrefix}-username`}>Username</Label>
        <Input
          id={`${idPrefix}-username`}
          autoComplete="off"
          maxLength={100}
          {...form.register("username")}
        />
        {errors.username ? (
          <p className="text-xs text-destructive">{errors.username.message}</p>
        ) : null}
      </div>
      {includePassword ? (
        <div className="grid gap-1.5">
          <Label htmlFor={`${idPrefix}-password`}>Password</Label>
          <Input
            id={`${idPrefix}-password`}
            type="password"
            autoComplete="new-password"
            maxLength={200}
            {...form.register("password")}
          />
          {errors.password ? (
            <p className="text-xs text-destructive">{errors.password.message}</p>
          ) : null}
        </div>
      ) : null}
    </>
  );
}
