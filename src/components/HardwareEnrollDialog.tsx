import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
  useCancelDeviceEnrollment,
  useCardReaderStatus,
  useDeviceEnrollmentOptions,
  useEnrollmentSession,
  useStartDeviceEnrollment,
  useTestCard,
} from "@/hooks/useEnrollments";
import { useUserDevicesQuery, useUsersForDeviceQuery } from "@/hooks/useUsers";
import type { CardTestResult, EnrollmentSession } from "@/types/enrollments";

type HardwareEnrollDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  fixedUser?: { id: string; username: string } | null;
};

function isCardType(enrollType: string): boolean {
  return (
    enrollType === "read_only_card" ||
    enrollType === "smart_card" ||
    enrollType === "biometric_then_card"
  );
}

function isProximityReader(reader: string | null | undefined): boolean {
  return reader === "EM Prox" || reader === "HID Prox";
}

function presentCopy(enrollType: string): string {
  if (enrollType === "face") {
    return "Please look at the Matrix reader.";
  }
  if (
    enrollType === "biometric" ||
    enrollType === "duress_finger" ||
    enrollType === "biometric_then_card"
  ) {
    return "Please present your finger on the Matrix reader.";
  }
  return "Hold the card flat on the card reader on the door until it accepts it.";
}

function sessionMessage(session: EnrollmentSession): string {
  switch (session.status) {
    case "starting":
    case "processing":
      return "Starting the reader.";
    case "waiting_for_card":
      return presentCopy(session.enrollType);
    case "verifying":
    case "saving":
      return "Checking that the device stored the credential.";
    case "success":
      return "Enrollment saved.";
    case "timeout":
      return "Enrollment timed out. Please try again.";
    case "cancelled":
      return "Cancelled in this app. The reader was not remotely cancelled.";
    case "persistence_failed":
      return "The device enrolled the credential, but it could not be saved here.";
    case "busy":
      return "This device is already enrolling someone.";
    case "failed":
      return session.errorCode
        ? asErrorMessage(session.errorCode)
        : "Enrollment failed.";
    default:
      return "Enrollment is in progress.";
  }
}

export function HardwareEnrollDialog({
  open,
  onOpenChange,
  fixedUser,
}: HardwareEnrollDialogProps) {
  const { toast } = useToast();
  const queryClient = useQueryClient();
  const devicesQuery = useDevicesQuery(open);
  const assignedDevices = useUserDevicesQuery(open && fixedUser ? fixedUser.id : null);
  const [deviceId, setDeviceId] = useState("");
  const [userId, setUserId] = useState(fixedUser?.id ?? "");
  const [enrollType, setEnrollType] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [cardTest, setCardTest] = useState<CardTestResult | null>(null);

  const options = useDeviceEnrollmentOptions(open && deviceId ? deviceId : null);
  const readerStatus = useCardReaderStatus(open && deviceId ? deviceId : null);
  const assignedUsers = useUsersForDeviceQuery(
    open && deviceId && !fixedUser ? deviceId : null,
  );
  const startEnrollment = useStartDeviceEnrollment();
  const sessionQuery = useEnrollmentSession(sessionId);
  const cancelSession = useCancelDeviceEnrollment();
  const testCard = useTestCard();
  const session = sessionQuery.data;
  const proximityReader = isProximityReader(readerStatus.data?.reader);
  const cardTestRequired = isCardType(enrollType) && !proximityReader;
  const waiting = Boolean(
    session &&
      (session.status === "starting" ||
        session.status === "waiting_for_card" ||
        session.status === "processing" ||
        session.status === "verifying" ||
        session.status === "saving"),
  );

  useEffect(() => {
    if (fixedUser) {
      setUserId(fixedUser.id);
    }
  }, [fixedUser]);

  useEffect(() => {
    if (session?.status !== "success") {
      return;
    }
    void queryClient.invalidateQueries({ queryKey: ["enrollments"] });
    void queryClient.invalidateQueries({ queryKey: ["credentials"] });
  }, [queryClient, session?.status, session?.id]);

  function reset() {
    setDeviceId("");
    setUserId(fixedUser?.id ?? "");
    setEnrollType("");
    setSessionId(null);
    setCardTest(null);
  }

  function close() {
    if (sessionId && waiting) {
      void cancelSession.mutateAsync(sessionId).catch(() => undefined);
    }
    reset();
    onOpenChange(false);
  }

  async function onStart() {
    if (!deviceId || !userId || !enrollType) {
      return;
    }
    if (cardTestRequired && cardTest?.outcome !== "success") {
      return;
    }
    try {
      const started = await startEnrollment.mutateAsync({
        deviceId,
        userId,
        enrollType,
      });
      setSessionId(started.id);
      setCardTest(null);
    } catch (reason: unknown) {
      toast({
        title: "Could not start enrollment",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  async function onTestCard() {
    if (!deviceId) {
      return;
    }
    setCardTest(null);
    try {
      const result = await testCard.mutateAsync(deviceId);
      setCardTest(result);
    } catch (reason: unknown) {
      toast({
        title: "Could not test the card",
        description: asErrorMessage(reason),
        variant: "destructive",
      });
    }
  }

  const deviceChoices = fixedUser
    ? (assignedDevices.data ?? []).map((assignment) => ({
        id: assignment.deviceId,
        label: `${assignment.deviceName} (${assignment.host}:${assignment.port})`,
      }))
    : (devicesQuery.data ?? [])
        .filter((device) => device.status === "active")
        .map((device) => ({
          id: device.id,
          label: `${device.deviceName} (${device.host}:${device.port})`,
        }));
  const assignedUserChoices = (assignedUsers.data ?? []).filter(
    (user) => user.status === "active",
  );

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          close();
          return;
        }
        onOpenChange(next);
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{fixedUser ? "Enroll credential" : "Add enrollment"}</DialogTitle>
          <DialogDescription>
            {fixedUser
              ? `${fixedUser.username}. Choose the type this device supports, then enroll. Present the card or face on the reader when it asks.`
              : "Choose the device, then the type it supports. Enroll starts the reader so you can present a card, face, or finger."}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3">
          <div className="grid gap-1.5">
            <Label>Device</Label>
            <Select
              value={deviceId || undefined}
              onValueChange={(value) => {
                setDeviceId(value);
                setEnrollType("");
                setCardTest(null);
                if (!fixedUser) {
                  setUserId("");
                }
              }}
              disabled={waiting}
            >
              <SelectTrigger aria-label="Enrollment device">
                <SelectValue placeholder="Select device" />
              </SelectTrigger>
              <SelectContent>
                {deviceChoices.map((device) => (
                  <SelectItem key={device.id} value={device.id}>
                    {device.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {deviceId ? (
            <div className="grid gap-1 rounded-md border border-border p-3 text-sm">
              <p className="font-medium">Card reader</p>
              <p>
                <span className="text-muted-foreground">Reader: </span>
                {readerStatus.data?.readerLabel ??
                  (readerStatus.isLoading ? "Checking the device…" : "Not reported")}
              </p>
              <p>
                <span className="text-muted-foreground">Smart card: </span>
                {readerStatus.data?.cardTypeLabel ?? "Not reported"}
              </p>
              <p>
                <span className="text-muted-foreground">Identifier: </span>
                {readerStatus.data?.identifierLabel ?? "Not reported"}
              </p>
              <p>
                <span className="text-muted-foreground">Status: </span>
                {testCard.isPending
                  ? "Present your card on the reader…"
                  : (cardTest?.message ?? readerStatus.data?.message ?? "Ready")}
              </p>
              {readerStatus.isError ? (
                <p className="text-xs text-destructive">
                  {asErrorMessage(readerStatus.error)}
                </p>
              ) : null}
              {cardTest?.outcome === "success" && cardTest.cardTypeLabel ? (
                <p>
                  <span className="text-muted-foreground">Card type: </span>
                  {cardTest.cardTypeLabel}
                </p>
              ) : null}
              {cardTest?.outcome === "success" && cardTest.cardNumber ? (
                <p>
                  <span className="text-muted-foreground">Card number: </span>
                  {cardTest.cardNumber}
                </p>
              ) : null}
            </div>
          ) : null}
          <div className="grid gap-1.5">
            <Label>Enrollment type</Label>
            <Select
              value={enrollType || undefined}
              onValueChange={setEnrollType}
              disabled={!deviceId || options.isLoading || options.isError || waiting}
            >
              <SelectTrigger aria-label="Enrollment type">
                <SelectValue
                  placeholder={
                    !deviceId
                      ? "Select a device first"
                      : options.isLoading
                        ? "Checking what this device supports…"
                        : "Select card, face, or another type"
                  }
                />
              </SelectTrigger>
              <SelectContent>
                {(options.data?.options ?? []).map((option) => (
                  <SelectItem key={option.enrollType} value={option.enrollType}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {options.isError ? (
              <p className="text-xs text-destructive">{asErrorMessage(options.error)}</p>
            ) : null}
            {deviceId &&
            options.isSuccess &&
            (options.data?.options.length ?? 0) === 0 ? (
              <p className="text-xs text-muted-foreground">
                This device did not report a card, face, or finger enrollment type.
              </p>
            ) : null}
          </div>
          {fixedUser ? (
            <p className="text-sm">
              <span className="text-muted-foreground">User: </span>
              {fixedUser.username}
            </p>
          ) : (
            <div className="grid gap-1.5">
              <Label>User</Label>
              <Select
                value={userId || undefined}
                onValueChange={setUserId}
                disabled={!deviceId || waiting}
              >
                <SelectTrigger aria-label="Assigned user">
                  <SelectValue placeholder="Select assigned user" />
                </SelectTrigger>
                <SelectContent>
                  {assignedUserChoices.map((user) => (
                    <SelectItem key={user.userId} value={user.userId}>
                      {user.username}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {deviceId &&
              !assignedUsers.isLoading &&
              assignedUserChoices.length === 0 ? (
                <p className="text-xs text-muted-foreground">
                  Assign a user to this device before enrollment.
                </p>
              ) : null}
            </div>
          )}
          {session ? (
            <div className="grid gap-1 text-sm">
              <p>{sessionMessage(session)}</p>
              {session.status === "success" && session.cardTypeLabel ? (
                <p>
                  <span className="text-muted-foreground">Card type: </span>
                  {session.cardTypeLabel}
                </p>
              ) : null}
              {session.status === "success" && session.cardNumber ? (
                <p>
                  <span className="text-muted-foreground">Card number: </span>
                  {session.cardNumber}
                </p>
              ) : null}
              {session.status === "success" && session.identifierTypeLabel ? (
                <p>
                  <span className="text-muted-foreground">Identifier: </span>
                  {session.identifierTypeLabel}
                </p>
              ) : null}
              {session.status === "success" &&
              isCardType(session.enrollType) &&
              session.identifierUnavailable ? (
                <p className="text-muted-foreground">
                  The device did not return a card number. The enrollment link was still saved.
                </p>
              ) : null}
            </div>
          ) : null}
          {cardTest && cardTest.outcome !== "success" && cardTest.outcome !== "proximity" ? (
            <p className="text-sm text-destructive">{cardTest.message}</p>
          ) : null}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={close}>
            {waiting ? "Cancel" : "Close"}
          </Button>
          {cardTestRequired && !waiting ? (
            <Button
              type="button"
              variant="secondary"
              disabled={!deviceId || !readerStatus.data?.supported || testCard.isPending}
              onClick={() => void onTestCard()}
            >
              {testCard.isPending ? "Present your card…" : "Test card"}
            </Button>
          ) : null}
          <Button
            type="button"
            disabled={
              !deviceId ||
              !userId ||
              !enrollType ||
              waiting ||
              startEnrollment.isPending ||
              (cardTestRequired && cardTest?.outcome !== "success")
            }
            onClick={() => void onStart()}
          >
            {waiting ? "Waiting for the reader…" : "Enroll on device"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
