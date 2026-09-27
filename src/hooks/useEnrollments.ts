import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  cancelEnrollment,
  createEnrollment,
  deviceEnrollmentOptions,
  enrollmentErrorMessage,
  enrollOnDevice,
  listEnrollments,
  retryEnrollment,
  revokeEnrollment,
} from "@/services/enrollments";
import type {
  CreateEnrollmentInput,
  Enrollment,
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
