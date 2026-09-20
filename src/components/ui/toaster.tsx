import * as React from "react";
import type { ComponentPropsWithoutRef } from "react";
import {
  Toast,
  ToastClose,
  ToastDescription,
  ToastProvider,
  ToastTitle,
  ToastViewport,
} from "@/components/ui/toast";

type ToastMessage = {
  id: string;
  title: string;
  description?: string;
  variant?: "default" | "destructive" | "success";
};

type ToastContextValue = {
  toast: (message: Omit<ToastMessage, "id">) => void;
};

const ToastContext = React.createContext<ToastContextValue | null>(null);

export function useToast() {
  const context = React.useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used within ToasterProvider");
  }
  return context;
}

export function ToasterProvider({ children }: { children: React.ReactNode }) {
  const [messages, setMessages] = React.useState<ToastMessage[]>([]);

  const toast = React.useCallback((message: Omit<ToastMessage, "id">) => {
    const id = crypto.randomUUID();
    setMessages((current) => [...current, { ...message, id }]);
  }, []);

  return (
    <ToastContext.Provider value={{ toast }}>
      <ToastProvider swipeDirection="right">
        {children}
        {messages.map((message) => (
          <ToastItem
            key={message.id}
            message={message}
            onOpenChange={(open) => {
              if (!open) {
                setMessages((current) =>
                  current.filter((item) => item.id !== message.id),
                );
              }
            }}
          />
        ))}
        <ToastViewport />
      </ToastProvider>
    </ToastContext.Provider>
  );
}

function ToastItem({
  message,
  onOpenChange,
}: {
  message: ToastMessage;
  onOpenChange: ComponentPropsWithoutRef<typeof Toast>["onOpenChange"];
}) {
  return (
    <Toast
      open
      duration={3500}
      variant={message.variant}
      onOpenChange={onOpenChange}
    >
      <div className="grid gap-0.5">
        <ToastTitle>{message.title}</ToastTitle>
        {message.description ? (
          <ToastDescription>{message.description}</ToastDescription>
        ) : null}
      </div>
      <ToastClose />
    </Toast>
  );
}
