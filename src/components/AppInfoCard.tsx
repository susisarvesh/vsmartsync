import type { AppInfo } from "../types/app";

type AppInfoCardProps = {
  info: AppInfo;
};

export function AppInfoCard({ info }: AppInfoCardProps) {
  return (
    <dl className="info-card">
      <div>
        <dt>Name</dt>
        <dd>{info.name}</dd>
      </div>
      <div>
        <dt>Version</dt>
        <dd>{info.version}</dd>
      </div>
      <div>
        <dt>Stage</dt>
        <dd>{info.stage}</dd>
      </div>
      <div>
        <dt>Description</dt>
        <dd>{info.description}</dd>
      </div>
    </dl>
  );
}
