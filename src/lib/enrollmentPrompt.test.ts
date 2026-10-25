import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { twoStepPrompt } from "./enrollmentPrompt.ts";

describe("twoStepPrompt", () => {
  it("follows captureFactor for card then face", () => {
    assert.equal(
      twoStepPrompt("card_then_face", "card", "HID Prox"),
      "Step 1 of 2: Hold the card flat on the card reader on the door until it accepts it.",
    );
    assert.equal(
      twoStepPrompt("card_then_face", "face", "MIFARE"),
      "Step 2 of 2: Please look at the Matrix reader.",
    );
  });

  it("follows captureFactor for face then card", () => {
    assert.equal(
      twoStepPrompt("face_then_card", "face", null),
      "Step 1 of 2: Please look at the Matrix reader.",
    );
    assert.equal(
      twoStepPrompt("face_then_card", "card", "MIFARE"),
      "Step 2 of 2: Present your MIFARE card on the reader.",
    );
  });
});
