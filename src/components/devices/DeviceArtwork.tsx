type DeviceFamily = "face" | "door" | "controller" | "unknown";

export function deviceFamily(model: string | null | undefined): DeviceFamily {
  const text = (model ?? "").toLowerCase();
  if (!text.trim()) {
    return "unknown";
  }
  if (text.includes("face")) {
    return "face";
  }
  if (
    text.includes("controller") ||
    text.includes("vega") ||
    text.includes("path") ||
    /\barc\b/.test(text)
  ) {
    return "controller";
  }
  if (
    text.includes("door") ||
    text.includes("ngt") ||
    text.includes("pvr") ||
    text.includes("fmx") ||
    text.includes("wireless") ||
    text.includes("direct")
  ) {
    return "door";
  }
  return "unknown";
}

const FAMILY_LABEL: Record<DeviceFamily, string> = {
  face: "Face reader",
  door: "Door reader",
  controller: "Controller",
  unknown: "Reader",
};

export function DeviceArtwork({
  model,
  className = "h-16 w-16",
}: {
  model: string | null | undefined;
  className?: string;
}) {
  const family = deviceFamily(model);
  return (
    <svg
      viewBox="0 0 64 64"
      className={className}
      role="img"
      aria-label={FAMILY_LABEL[family]}
    >
      <rect width="64" height="64" rx="8" fill="#eff6ff" />
      {family === "face" ? <FaceReader /> : null}
      {family === "door" ? <DoorReader /> : null}
      {family === "controller" ? <Controller /> : null}
      {family === "unknown" ? <UnknownReader /> : null}
    </svg>
  );
}

function FaceReader() {
  return (
    <>
      <rect x="18" y="8" width="28" height="48" rx="4" fill="#1e3a8a" />
      <circle cx="32" cy="18" r="4" fill="#93c5fd" />
      <rect x="22" y="26" width="20" height="16" rx="2" fill="#dbeafe" />
      <rect x="26" y="46" width="12" height="3" rx="1" fill="#93c5fd" />
    </>
  );
}

function DoorReader() {
  return (
    <>
      <rect x="22" y="6" width="20" height="52" rx="6" fill="#1e3a8a" />
      <circle cx="32" cy="20" r="6" fill="#dbeafe" />
      <circle cx="32" cy="20" r="2.5" fill="#1d4ed8" />
      <rect x="28" y="32" width="8" height="3" rx="1" fill="#93c5fd" />
      <rect x="28" y="38" width="8" height="3" rx="1" fill="#93c5fd" />
      <rect x="28" y="44" width="8" height="3" rx="1" fill="#93c5fd" />
    </>
  );
}

function Controller() {
  return (
    <>
      <rect x="8" y="16" width="48" height="32" rx="3" fill="#1e3a8a" />
      <circle cx="16" cy="26" r="2" fill="#86efac" />
      <circle cx="22" cy="26" r="2" fill="#fde68a" />
      <rect x="14" y="34" width="8" height="6" rx="1" fill="#dbeafe" />
      <rect x="26" y="34" width="8" height="6" rx="1" fill="#dbeafe" />
      <rect x="38" y="34" width="10" height="6" rx="1" fill="#93c5fd" />
    </>
  );
}

function UnknownReader() {
  return (
    <>
      <rect x="16" y="10" width="32" height="44" rx="4" fill="#334155" />
      <rect x="22" y="18" width="20" height="14" rx="2" fill="#e2e8f0" />
      <circle cx="32" cy="42" r="3" fill="#93c5fd" />
    </>
  );
}
