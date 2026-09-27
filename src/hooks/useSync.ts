import { useMutation, useQueryClient } from "@tanstack/react-query";
import { syncAssignedUsers, syncErrorMessage } from "@/services/sync";

export function useSyncAssignedUsers() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      deviceId,
      userIds,
    }: {
      deviceId: string;
      userIds: string[];
    }) => syncAssignedUsers(deviceId, userIds),
    onSuccess: (_result, variables) => {
      void queryClient.invalidateQueries({
        queryKey: ["users-for-device", variables.deviceId],
      });
    },
  });
}

export function asSyncErrorMessage(reason: unknown): string {
  if (typeof reason === "string") {
    return syncErrorMessage(reason);
  }
  if (reason instanceof Error) {
    return syncErrorMessage(reason.message);
  }
  if (reason && typeof reason === "object" && "message" in reason) {
    const message = (reason as { message: unknown }).message;
    if (typeof message === "string") {
      return syncErrorMessage(message);
    }
  }
  return syncErrorMessage("");
}
