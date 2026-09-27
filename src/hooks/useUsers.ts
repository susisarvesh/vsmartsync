import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  activateUser,
  assignUserDevice,
  createUser,
  deactivateUser,
  deleteUser,
  listUserDevices,
  listUsers,
  listUsersForDevice,
  removeUserDevice,
  updateUser,
  userErrorMessage,
} from "@/services/users";
import type { User } from "@/types/users";

function asErrorMessage(reason: unknown): string {
  if (typeof reason === "string") {
    return userErrorMessage(reason);
  }
  if (reason instanceof Error) {
    return userErrorMessage(reason.message);
  }
  if (reason && typeof reason === "object" && "message" in reason) {
    const message = (reason as { message: unknown }).message;
    if (typeof message === "string") {
      return userErrorMessage(message);
    }
  }
  return userErrorMessage("");
}

export function useUsersQuery(enabled: boolean) {
  return useQuery({
    queryKey: ["users"],
    queryFn: listUsers,
    enabled,
  });
}

export function useCreateUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (username: string) => createUser(username),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
    meta: { errorMapper: asErrorMessage },
  });
}

export function useUpdateUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, username }: { id: string; username: string }) =>
      updateUser(id, username),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
  });
}

export function useActivateUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => activateUser(id),
    onSuccess: (user: User) => {
      queryClient.setQueryData<User[]>(["users"], (current) =>
        current?.map((item) => (item.id === user.id ? user : item)),
      );
      void queryClient.invalidateQueries({ queryKey: ["users"] });
    },
  });
}

export function useDeactivateUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deactivateUser(id),
    onSuccess: (user: User) => {
      queryClient.setQueryData<User[]>(["users"], (current) =>
        current?.map((item) => (item.id === user.id ? user : item)),
      );
      void queryClient.invalidateQueries({ queryKey: ["users"] });
    },
  });
}

export function useDeleteUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteUser(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["users"] });
      void queryClient.invalidateQueries({ queryKey: ["user-devices"] });
      void queryClient.invalidateQueries({ queryKey: ["users-for-device"] });
      void queryClient.invalidateQueries({ queryKey: ["enrollments"] });
      void queryClient.invalidateQueries({ queryKey: ["credentials"] });
    },
  });
}

export function useUserDevicesQuery(userId: string | null) {
  return useQuery({
    queryKey: ["user-devices", userId],
    queryFn: () => listUserDevices(userId ?? ""),
    enabled: Boolean(userId),
  });
}

export function useUsersForDeviceQuery(deviceId: string | null) {
  return useQuery({
    queryKey: ["users-for-device", deviceId],
    queryFn: () => listUsersForDevice(deviceId ?? ""),
    enabled: Boolean(deviceId),
  });
}

export function useAssignUserDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ userId, deviceId }: { userId: string; deviceId: string }) =>
      assignUserDevice(userId, deviceId),
    onSuccess: (assignment, variables) => {
      void queryClient.invalidateQueries({
        queryKey: ["user-devices", variables.userId],
      });
      void queryClient.invalidateQueries({
        queryKey: ["users-for-device", assignment.deviceId],
      });
    },
  });
}

export function useRemoveUserDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ userId, deviceId }: { userId: string; deviceId: string }) =>
      removeUserDevice(userId, deviceId),
    onSuccess: (_result, variables) => {
      void queryClient.invalidateQueries({
        queryKey: ["user-devices", variables.userId],
      });
      void queryClient.invalidateQueries({
        queryKey: ["users-for-device", variables.deviceId],
      });
    },
  });
}

export { asErrorMessage };
