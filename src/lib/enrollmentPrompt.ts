export function isTwoStepEnrollType(enrollType: string): boolean {
  return enrollType === "card_then_face" || enrollType === "face_then_card";
}

export function isCardEnrollType(enrollType: string): boolean {
  return (
    enrollType === "read_only_card" ||
    enrollType === "smart_card" ||
    enrollType === "biometric_then_card" ||
    isTwoStepEnrollType(enrollType)
  );
}

export function cardPresentCopy(reader: string | null | undefined): string {
  if (reader === "MIFARE") {
    return "Present your MIFARE card on the reader.";
  }
  return "Hold the card flat on the card reader on the door until it accepts it.";
}

/// Prompt for the factor the session is waiting on. Order follows the enroll type.
export function twoStepPrompt(
  enrollType: string,
  captureFactor: string | null | undefined,
  reader: string | null | undefined,
): string {
  const faceFirst = enrollType === "face_then_card";
  const factor =
    captureFactor === "face" || captureFactor === "card"
      ? captureFactor
      : faceFirst
        ? "face"
        : "card";
  const step = faceFirst
    ? factor === "face"
      ? 1
      : 2
    : factor === "card"
      ? 1
      : 2;
  const instruction =
    factor === "face"
      ? "Please look at the Matrix reader."
      : cardPresentCopy(reader);
  return `Step ${step} of 2: ${instruction}`;
}
