import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  acceptDatabasePrivacy,
  provisionDatabase,
  type DatabaseEngineChoice,
  type SetupStatus,
} from "@/services/setup";

const ENGINES: Array<{
  id: DatabaseEngineChoice;
  name: string;
  detail: string;
  install: string;
}> = [
  {
    id: "postgres",
    name: "PostgreSQL",
    detail: "Local server on port 5432. This is the engine the application uses for daily work.",
    install: "Install PostgreSQL 16 on this computer and start the service on port 5432.",
  },
  {
    id: "mysql",
    name: "MySQL",
    detail: "Local server on port 3306. Setup creates the database and tables when MySQL is already running.",
    install: "Install MySQL on this computer and start the service on port 3306.",
  },
  {
    id: "mssql",
    name: "SQL Server",
    detail: "Local server on port 1433. Setup creates the database and tables when SQL Server is already running.",
    install: "Install SQL Server on this computer and start it listening on port 1433.",
  },
];

type SetupWizardProps = {
  initial: SetupStatus;
  onReady: () => void;
};

export function SetupWizard({ initial, onReady }: SetupWizardProps) {
  const [status, setStatus] = useState(initial);
  const [engine, setEngine] = useState<DatabaseEngineChoice>(
    initial.engine ?? "postgres",
  );
  const [adminUser, setAdminUser] = useState("");
  const [adminPassword, setAdminPassword] = useState("");
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selected = ENGINES.find((item) => item.id === engine) ?? ENGINES[0];

  async function accept() {
    setWorking(true);
    setError(null);
    try {
      setStatus(await acceptDatabasePrivacy());
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setWorking(false);
    }
  }

  async function setup() {
    setWorking(true);
    setError(null);
    try {
      const next = await provisionDatabase({
        engine,
        adminUser: adminUser.trim() || undefined,
        adminPassword: adminPassword || undefined,
      });
      setStatus(next);
      setAdminPassword("");
      if (next.ready) {
        onReady();
      }
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setWorking(false);
    }
  }

  return (
    <div className="flex h-full items-center justify-center bg-slate-100 p-6">
      <div className="w-full max-w-2xl rounded-lg border border-slate-300 bg-white shadow-sm">
        <div className="border-b border-slate-200 px-6 py-5">
          <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
            Vsmart Sync setup
          </p>
          <h1 className="mt-1 text-xl font-semibold text-slate-900">
            {status.privacyAccepted ? "Choose the local database" : "Privacy notice"}
          </h1>
        </div>

        {status.privacyAccepted ? (
          <div className="space-y-4 px-6 py-5">
            <p className="text-sm text-slate-600">
              Choose the database server already installed on this computer.
              Vsmart Sync uses one database. It creates <code>vsmart_sync</code> and
              the application tables when that server accepts a local connection.
            </p>
            <div className="grid gap-2">
              {ENGINES.map((item) => (
                <label
                  key={item.id}
                  className={`cursor-pointer rounded-md border px-3 py-3 ${
                    engine === item.id
                      ? "border-blue-600 bg-blue-50"
                      : "border-slate-300 bg-white"
                  }`}
                >
                  <span className="flex items-start gap-3">
                    <input
                      type="radio"
                      name="database-engine"
                      className="mt-1"
                      checked={engine === item.id}
                      onChange={() => setEngine(item.id)}
                    />
                    <span>
                      <span className="block text-sm font-semibold text-slate-900">
                        {item.name}
                      </span>
                      <span className="mt-1 block text-sm text-slate-600">{item.detail}</span>
                    </span>
                  </span>
                </label>
              ))}
            </div>

            {status.needsAdminLogin ? (
              <div className="grid gap-3 rounded-md border border-slate-300 bg-slate-50 p-3 sm:grid-cols-2">
                <label className="grid gap-1 text-sm">
                  <span className="font-medium text-slate-800">Administrator login</span>
                  <input
                    value={adminUser}
                    onChange={(event) => setAdminUser(event.target.value)}
                    autoComplete="off"
                    className="h-9 rounded-md border border-slate-300 bg-white px-2"
                  />
                </label>
                <label className="grid gap-1 text-sm">
                  <span className="font-medium text-slate-800">Administrator password</span>
                  <input
                    type="password"
                    value={adminPassword}
                    onChange={(event) => setAdminPassword(event.target.value)}
                    autoComplete="off"
                    className="h-9 rounded-md border border-slate-300 bg-white px-2"
                  />
                </label>
                <p className="text-xs text-slate-500 sm:col-span-2">
                  Used once to create the Vsmart database. It is not saved.
                </p>
              </div>
            ) : null}

            {status.message ? (
              <p className="text-sm text-slate-700" role="status">
                {status.message}
              </p>
            ) : null}
            {!status.ready && status.message.toLowerCase().includes("not running") ? (
              <p className="text-sm text-slate-600">{selected.install}</p>
            ) : null}
            {error ? (
              <p className="text-sm text-red-700" role="alert">
                {error}
              </p>
            ) : null}

            <div className="flex justify-end">
              <Button type="button" disabled={working} onClick={() => void setup()}>
                {working ? "Setting up…" : `Set up ${selected.name}`}
              </Button>
            </div>
          </div>
        ) : (
          <div className="space-y-4 px-6 py-5 text-sm text-slate-700">
            <p>
              Vsmart Sync runs on this computer. It stores people, devices, credentials,
              and access events in the database you choose next. That database stays on
              this workstation.
            </p>
            <p>
              Device passwords and card or face enrollment data are encrypted on this
              computer so the app can talk to Matrix COSEC readers. They are not shown
              in the application screens and are not sent to a Vsmart cloud service by
              this program.
            </p>
            <p>
              The program does not sell this information and does not require an account
              with Vsmart to run. You choose PostgreSQL, MySQL, or SQL Server. Only one
              of those databases is used.
            </p>
            {error ? (
              <p className="text-red-700" role="alert">
                {error}
              </p>
            ) : null}
            <div className="flex justify-end">
              <Button type="button" disabled={working} onClick={() => void accept()}>
                {working ? "Saving…" : "Accept and continue"}
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
