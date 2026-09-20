import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createDevice,
  deviceErrorMessage,
  listDevices,
  setDevicePassword,
  testDeviceConnection,
  updateDevice,
} from "@/services/devices";
import type {
  CreateDeviceInput,
  Device,
  UpdateDeviceInput,
} from "@/types/devices";

function asErrorMessage(reason: unknown): string {
  if (typeof reason === "string") {
    return deviceErrorMessage(reason);
  }
  if (reason instanceof Error) {
    return deviceErrorMessage(reason.message);
  }
  if (reason && typeof reason === "object" && "message" in reason) {
    const message = (reason as { message: unknown }).message;
    if (typeof message === "string") {
      return deviceErrorMessage(message);
    }
  }
  return deviceErrorMessage("");
}

export function useDevicesQuery(enabled: boolean) {
  return useQuery({
    queryKey: ["devices"],
    queryFn: listDevices,
    enabled,
  });
}

export function useCreateDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateDeviceInput) => createDevice(input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["devices"] }),
  });
}

export function useUpdateDevice() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateDeviceInput) => updateDevice(input),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["devices"] }),
  });
}

export function useSetDevicePassword() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, password }: { id: string; password: string }) =>
      setDevicePassword(id, password),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["devices"] }),
  });
}

export function useTestDeviceConnection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => testDeviceConnection(id),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: ["devices"] });
    },
    onSuccess: (device: Device) => {
      queryClient.setQueryData<Device[]>(["devices"], (current) =>
        current?.map((item) => (item.id === device.id ? device : item)),
      );
    },
  });
}

export { asErrorMessage };
