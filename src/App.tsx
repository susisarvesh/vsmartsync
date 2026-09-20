import { useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "@/components/layout/AppShell";
import { ToasterProvider } from "@/components/ui/toaster";
import { useDatabaseStatus } from "@/hooks/useDatabaseStatus";
import type { AppRoute } from "@/navigation";
import { DashboardPage } from "@/pages/DashboardPage";
import { DevicesPage } from "@/pages/DevicesPage";
import { UsersPage } from "@/pages/UsersPage";
import "@/index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function isDesktopApp(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function AppContent() {
  const [route, setRoute] = useState<AppRoute>("dashboard");
  const database = useDatabaseStatus();
  const inDesktopApp = isDesktopApp();
  const databaseConnected = Boolean(database.status?.connected);

  if (!inDesktopApp) {
    return (
      <div className="mx-auto max-w-lg p-6">
        <h1 className="text-xl font-semibold">Vsmart Sync</h1>
        <p className="mt-2 text-sm text-destructive" role="alert">
          Vsmart Sync is a local desktop app. Close this browser tab and run{" "}
          <code>npm install && npm start</code> from the project folder, then
          use the Vsmart Sync window.
        </p>
      </div>
    );
  }

  return (
    <AppShell
      route={route}
      onNavigate={setRoute}
      database={database.status}
      databaseLoading={database.loading}
      databaseError={database.error}
      retrying={database.retrying}
      onRetryDatabase={database.retry}
    >
      {route === "dashboard" ? (
        <DashboardPage
          database={database.status}
          databaseLoading={database.loading}
          databaseError={database.error}
          retrying={database.retrying}
          onRetryDatabase={database.retry}
          onOpenUsers={() => setRoute("users")}
        />
      ) : null}
      {route === "users" ? <UsersPage enabled={databaseConnected} /> : null}
      {route === "devices" ? (
        <DevicesPage enabled={databaseConnected} />
      ) : null}
    </AppShell>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ToasterProvider>
        <AppContent />
      </ToasterProvider>
    </QueryClientProvider>
  );
}
