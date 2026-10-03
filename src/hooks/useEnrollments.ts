import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  cancelDeviceEnrollmentSession,
  cancelEnrollment,
  createEnrollment,
  deviceEnrollmentOptions,
  enrollmentErrorMessage,
  enrollOnDevice,
  getCardReaderStatus,
  getDeviceEnrollmentSession,
  listEnrollments,
  readCard,
  testCard,
  retryEnrollment,
  revokeEnrollment,
  startDeviceEnrollment,
} from "@/services/enrollments";
import type {
  CreateEnrollmentInput,
  Enrollment,
  EnrollmentSessionStatus,
  ListEnrollmentsFilter,
} from "@/types/enrollments";

function asErrorMessage(reason: unknown): string {
  if (typeof reason === "string") {
    return enrollmentErrorMessage(reason);
  }
  if (reason instanceof Error) {
    return enrollmentErrorMessage(reason.message);
  }
  if (reason && typeof reason === "object" && "message" in reason) {
    const message = (reason as { message: unknown }).message;
    if (typeof message === "string") {
      return enrollmentErrorMessage(message);
    }
  }
  return enrollmentErrorMessage("");
}

export function useEnrollmentsQuery(
  enabled: boolean,
  filter: ListEnrollmentsFilter = {},
) {
  return useQuery({
    queryKey: ["enrollments", filter],
    queryFn: () => listEnrollments(filter),
    enabled,
  });
}

export function useCreateEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateEnrollmentInput) => createEnrollment(input),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["enrollments"] }),
  });
}

export function useCancelEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => cancelEnrollment(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["enrollments"] }),
  });
}

export function useRevokeEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => revokeEnrollment(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["enrollments"] }),
  });
}

export function useRetryEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => retryEnrollment(id),
    onSuccess: (enrollment: Enrollment) => {
      queryClient.setQueryData<Enrollment[]>(["enrollments"], (current) =>
        current?.map((item) =>
          item.id === enrollment.id ? enrollment : item,
        ),
      );
      void queryClient.invalidateQueries({ queryKey: ["enrollments"] });
    },
  });
}

const TERMINAL_SESSION: EnrollmentSessionStatus[] = [
  "success",
  "failed",
  "cancelled",
  "timeout",
  "busy",
  "persistence_failed",
];

export function useStartDeviceEnrollment() {
  return useMutation({
    mutationFn: (input: {
      deviceId: string;
      userId: string;
      enrollType: string;
    }) => startDeviceEnrollment(input.deviceId, input.userId, input.enrollType),
  });
}

export function useEnrollmentSession(sessionId: string | null) {
  return useQuery({
    queryKey: ["enrollment-session", sessionId],
    queryFn: () => getDeviceEnrollmentSession(sessionId ?? ""),
    enabled: Boolean(sessionId),
    refetchInterval: (query) => {
      const status = query.state.data?.status;
      if (status && TERMINAL_SESSION.includes(status)) {
        return false;
      }
      return 1000;
    },
  });
}

export function useCancelDeviceEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => cancelDeviceEnrollmentSession(id),
    onSuccess: (session) => {
      queryClient.setQueryData(["enrollment-session", session.id], session);
      void queryClient.invalidateQueries({ queryKey: ["enrollments"] });
    },
  });
}

export function useReadCard() {
  return useMutation({
    mutationFn: (deviceId: string) => readCard(deviceId),
  });
}

export function useCardReaderStatus(deviceId: string | null) {
  return useQuery({
    queryKey: ["card-reader-status", deviceId],
    queryFn: () => getCardReaderStatus(deviceId ?? ""),
    enabled: Boolean(deviceId),
  });
}

export function useTestCard() {
  return useMutation({
    mutationFn: (deviceId: string) => testCard(deviceId),
  });
}

export function useEnrollOnDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      deviceId: string;
      userId: string;
      enrollType: string;
    }) => enrollOnDevice(input.deviceId, input.userId, input.enrollType),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["enrollments"] });
      void queryClient.invalidateQueries({ queryKey: ["credentials"] });
    },
  });
}

export function useDeviceEnrollmentOptions(deviceId: string | null) {
  return useQuery({
    queryKey: ["device-enrollment-options", deviceId],
    queryFn: () => deviceEnrollmentOptions(deviceId ?? ""),
    enabled: Boolean(deviceId),
  });
}

export { asErrorMessage };
