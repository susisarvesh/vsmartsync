import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createUser,
  deactivateUser,
  listUsers,
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
    mutationFn: (name: string) => createUser(name),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
    meta: { errorMapper: asErrorMessage },
  });
}

export function useUpdateUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      updateUser(id, name),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
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

export { asErrorMessage };
