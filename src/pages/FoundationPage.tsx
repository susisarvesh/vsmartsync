import { AppInfoCard } from "../components/AppInfoCard";
import { useAppInfo } from "../hooks/useAppInfo";

export function FoundationPage() {
  const { info, error, loading } = useAppInfo();

  return (
    <main className="container">
      <h1>Matrixcosec</h1>
      <p className="lede">
        Project foundation only. This screen verifies React → Tauri → Rust.
      </p>
      {loading && <p>Loading application info from Rust…</p>}
      {error && (
        <p className="error" role="alert">
          Failed to invoke <code>get_app_info</code>: {error}
        </p>
      )}
      {info && <AppInfoCard info={info} />}
    </main>
  );
}
