import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useToast } from "@/components/ui/toaster";
import {
  asErrorMessage,
  useCreateDevice,
  useTestDeviceConnection,
} from "@/hooks/useDevices";
import { scanMatrixDevices } from "@/services/devices";
import type { Device, DiscoveredDevice } from "@/types/devices";

type NetworkScanDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  existing: Device[];
};

function Radar() {
  return (
    <div
      className="relative mx-auto h-36 w-36"
      role="img"
      aria-label="Scanning the network"
    >
      <style>{`
        @keyframes radar-orbit {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        .radar-orbit { animation: radar-orbit 2.4s linear infinite; }
        @media (prefers-reduced-motion: reduce) {
          .radar-orbit { animation: none; }
        }
      `}</style>
      <div className="absolute inset-0 rounded-full border border-primary/30 bg-primary/5" />
      <div className="absolute inset-5 rounded-full border border-primary/25" />
      <div className="absolute inset-10 rounded-full border border-primary/20" />
      <div className="absolute left-1/2 top-1/2 h-1.5 w-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full bg-primary" />
      <div className="radar-orbit absolute inset-0">
        <div className="absolute left-1/2 top-0 h-1/2 w-px -translate-x-1/2 bg-primary/70" />
        <div className="absolute left-1/2 top-0 h-2.5 w-2.5 -translate-x-1/2 rounded-full bg-primary" />
      </div>
    </div>
  );
}

function alreadyRegistered(device: DiscoveredDevice, existing: Device[]): boolean {
  return existing.some(
    (item) => item.host === device.host && item.port === device.port,
  );
}

export function NetworkScanDialog({
  open,
  onOpenChange,
  existing,
}: NetworkScanDialogProps) {
  const { toast } = useToast();
  const createDevice = useCreateDevice();
  const testConnection = useTestDeviceConnection();
  const [scanning, setScanning] = useState(false);
  const [failed, setFailed] = useState(false);
  const [found, setFound] = useState<DiscoveredDevice[]>([]);
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [connecting, setConnecting] = useState(false);

  useEffect(() => {
    if (!open) {
      return;
    }
    let cancelled = false;
    setScanning(true);
    setFailed(false);
    setFound([]);
    setUsername("admin");
    setPassword("");
    void scanMatrixDevices()
      .then((devices) => {
        if (!cancelled) {
          setFound(devices);
          setScanning(false);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFailed(true);
          setScanning(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  const fresh = found.filter((device) => !alreadyRegistered(device, existing));

  async function onConnect() {
    if (!username.trim() || !password) {
      return;
    }
    setConnecting(true);
    let connected = 0;
    let rejected = 0;
    try {
      for (const device of fresh) {
        const created = await createDevice.mutateAsync({
          deviceName: device.name?.trim() || `Matrix ${device.host}`,
          host: device.host,
          port: device.port,
          macAddress: "",
          username: username.trim(),
          password,
        });
        try {
          const tested = await testConnection.mutateAsync(created.id);
          if (tested.connectionStatus === "online") {
            connected += 1;
          } else {
            rejected += 1;
          }
        } catch {
          rejected += 1;
        }
      }
      toast({
        title: connected > 0 ? "Devices connected" : "Devices saved",
        description:
          rejected > 0
            ? `${connected} online. ${rejected} saved, but the password was rejected or the device did not answer.`
            : `${connected} Matrix device${connected === 1 ? "" : "s"} added from the network.`,
        variant: connected > 0 && rejected === 0 ? "success" : "default",
      });
      onOpenChange(false);
    } catch (reason: unknown) {
      toast({
        title: "Could not add a device",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    } finally {
      setConnecting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Auto scan</DialogTitle>
          <DialogDescription>
            Looks for Matrix devices on this network. You do not enter an IP
            address. Manual add stays available.
          </DialogDescription>
        </DialogHeader>
        {scanning ? (
          <div className="grid gap-3 py-2">
            <Radar />
            <p className="text-center text-sm text-muted-foreground">
              Scanning for Matrix devices…
            </p>
          </div>
        ) : null}
        {failed ? (
          <p className="text-sm text-destructive">
            The network scan could not start.
          </p>
        ) : null}
        {!scanning && !failed && found.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No Matrix devices answered on this network.
          </p>
        ) : null}
        {!scanning && found.length > 0 ? (
          <div className="grid gap-3">
            <ul className="grid max-h-48 gap-1 overflow-auto text-sm">
              {found.map((device) => {
                const known = alreadyRegistered(device, existing);
                return (
                  <li
                    key={`${device.host}:${device.port}`}
                    className="flex items-center justify-between gap-2 rounded-md border border-border px-3 py-2"
                  >
                    <span>
                      <span className="font-medium">
                        {device.name?.trim() || "Matrix device"}
                      </span>
                      <span className="text-muted-foreground">
                        {" "}
                        {device.host}:{device.port}
                      </span>
                    </span>
                    {known ? (
                      <span className="text-xs text-muted-foreground">
                        Already added
                      </span>
                    ) : null}
                  </li>
                );
              })}
            </ul>
            {fresh.length > 0 ? (
              <div className="grid gap-3">
                <p className="text-sm text-muted-foreground">
                  Enter the device login once. It is used for every new device
                  found.
                </p>
                <div className="grid gap-1.5">
                  <Label htmlFor="scan-username">Username</Label>
                  <Input
                    id="scan-username"
                    value={username}
                    autoComplete="off"
                    onChange={(event) => setUsername(event.target.value)}
                    disabled={connecting}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor="scan-password">Password</Label>
                  <Input
                    id="scan-password"
                    type="password"
                    value={password}
                    autoComplete="new-password"
                    onChange={(event) => setPassword(event.target.value)}
                    disabled={connecting}
                  />
                </div>
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">
                Every device found is already registered.
              </p>
            )}
          </div>
        ) : null}
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={connecting}
          >
            Close
          </Button>
          {!scanning && fresh.length > 0 ? (
            <Button
              type="button"
              disabled={connecting || !username.trim() || !password}
              onClick={() => void onConnect()}
            >
              {connecting ? "Connecting…" : `Connect ${fresh.length}`}
            </Button>
          ) : null}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
