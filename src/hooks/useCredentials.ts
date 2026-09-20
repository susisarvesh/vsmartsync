import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createCredential,
  credentialErrorMessage,
  listCredentials,
  setCredentialStatus,
  updateCredential,
} from "@/services/credentials";
import type {
  CreateCredentialInput,
  Credential,
  ListCredentialsFilter,
} from "@/types/credentials";

function asErrorMessage(reason: unknown): string {
  if (typeof reason === "string") {
    return credentialErrorMessage(reason);
  }
  if (reason instanceof Error) {
    return credentialErrorMessage(reason.message);
  }
  if (reason && typeof reason === "object" && "message" in reason) {
    const message = (reason as { message: unknown }).message;
    if (typeof message === "string") {
      return credentialErrorMessage(message);
    }
  }
  return credentialErrorMessage("");
}

export function useCredentialsQuery(
  enabled: boolean,
  filter: ListCredentialsFilter = {},
) {
  return useQuery({
    queryKey: ["credentials", filter],
    queryFn: () => listCredentials(filter),
    enabled,
  });
}

export function useCreateCredential() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateCredentialInput) => createCredential(input),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["credentials"] }),
  });
}

export function useUpdateCredential() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, value }: { id: string; value: string }) =>
      updateCredential(id, value),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["credentials"] }),
  });
}

export function useSetCredentialStatus() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      status,
    }: {
      id: string;
      status: "active" | "inactive";
    }) => setCredentialStatus(id, status),
    onSuccess: (credential: Credential) => {
      queryClient.setQueryData<Credential[]>(["credentials"], (current) =>
        current?.map((item) =>
          item.id === credential.id ? credential : item,
        ),
      );
      void queryClient.invalidateQueries({ queryKey: ["credentials"] });
    },
  });
}

export { asErrorMessage };
