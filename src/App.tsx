import { useEffect, useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "@/components/layout/AppShell";
import { SetupWizard } from "@/components/setup/SetupWizard";
import { ToasterProvider } from "@/components/ui/toaster";
import { useDatabaseStatus } from "@/hooks/useDatabaseStatus";
import { getSetupStatus, type SetupStatus } from "@/services/setup";
import type { AppRoute } from "@/navigation";
import { AccessLogsPage } from "@/pages/AccessLogsPage";
import { AssignDevicePage } from "@/pages/AssignDevicePage";
import { CredentialsPage } from "@/pages/CredentialsPage";
import { ImportUsersPage } from "@/pages/ImportUsersPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { DevicesPage } from "@/pages/DevicesPage";
import { UsersPage } from "@/pages/UsersPage";
import "@/index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
      staleTime: 30_000,
    },
  },
});

function isDesktopApp(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function AppContent() {
  const [route, setRoute] = useState<AppRoute>("dashboard");
  const [enrollmentUserId, setEnrollmentUserId] = useState<string | null>(null);

  function navigate(next: AppRoute, userId?: string | null) {
    setRoute(next);
    setEnrollmentUserId(next === "enrollment" ? (userId ?? null) : null);
  }
  const database = useDatabaseStatus();
  const inDesktopApp = isDesktopApp();
  const databaseConnected = Boolean(database.status?.connected);
  const [setup, setSetup] = useState<SetupStatus | null>(null);
  const [setupLoading, setSetupLoading] = useState(true);

  useEffect(() => {
    if (!inDesktopApp) {
      setSetupLoading(false);
      return;
    }
    let cancelled = false;
    getSetupStatus()
      .then((next) => {
        if (!cancelled) {
          setSetup(next);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSetup(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setSetupLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [inDesktopApp]);

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

  if (setupLoading || (setup && !setup.ready)) {
    return setup ? (
      <SetupWizard
        initial={setup}
        onReady={() => {
          setSetup({ ...setup, ready: true, privacyAccepted: true });
          void database.retry();
        }}
      />
    ) : (
      <div className="flex h-full items-center justify-center text-sm text-slate-600">
        Opening setup…
      </div>
    );
  }

  return (
    <AppShell
      route={route}
      onNavigate={navigate}
      database={database.status}
      databaseLoading={database.loading}
      databaseError={database.error}
      retrying={database.retrying}
      onRetryDatabase={database.retry}
    >
      {route === "dashboard" ? (
        <DashboardPage
          database={database.status}
          onNavigate={navigate}
        />
      ) : null}
      {route === "user-configuration" ? (
        <UsersPage
          enabled={databaseConnected}
          onEnroll={(userId) => navigate("enrollment", userId)}
        />
      ) : null}
      {route === "import-users" ? (
        <ImportUsersPage enabled={databaseConnected} />
      ) : null}
      {route === "enrollment" ? (
        <AssignDevicePage
          enabled={databaseConnected}
          focusUserId={enrollmentUserId}
        />
      ) : null}
      {route === "devices" ? (
        <DevicesPage enabled={databaseConnected} />
      ) : null}
      {route === "credentials" ? (
        <CredentialsPage enabled={databaseConnected} />
      ) : null}
      {route === "in-out-report" ? (
        <AccessLogsPage enabled={databaseConnected} />
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
