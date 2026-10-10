import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { fetchDeviceEvents, listAccessEvents } from "@/services/events";

export function useAccessEvents(enabled: boolean, deviceId: string | null) {
  return useQuery({
    queryKey: ["access-events", deviceId],
    queryFn: () => listAccessEvents(deviceId),
    enabled,
    refetchInterval: 30_000,
  });
}

export function useFetchDeviceEvents() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (deviceId: string) => fetchDeviceEvents(deviceId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["access-events"] });
    },
  });
}
