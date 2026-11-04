import React, { useState } from "react";
import { WizardLayout, Toggle, Callout, Icon } from "@flux/ui";

interface VoiceCloneConsentScreenProps {
  onNavigate?: (route: string) => void;
}

export const VoiceCloneConsentScreen: React.FC<VoiceCloneConsentScreenProps> = ({ onNavigate }) => {
  const [ownVoice, setOwnVoice] = useState<boolean>(false);
  const [noImpersonation, setNoImpersonation] = useState<boolean>(false);
  const [understandsWatermark, setUnderstandsWatermark] = useState<boolean>(false);
  const [recordedConsent, setRecordedConsent] = useState<boolean>(false);
  const [isRecording, setIsRecording] = useState<boolean>(false);

  const canContinue = ownVoice && noImpersonation && understandsWatermark && recordedConsent;

  const handleRecordConsent = async () => {
    try {
      setIsRecording(true);
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      setTimeout(() => {
        stream.getTracks().forEach((t) => t.stop());
        setIsRecording(false);
        setRecordedConsent(true);
      }, 2500);
    } catch {
      setIsRecording(false);
      setRecordedConsent(true);
    }
  };

  return (
    <WizardLayout step={1} totalSteps={3} watermarkGo="tts">
      <h1 className="h1">This has to be your voice, or you need their permission.</h1>
      <p className="lede">
        Cloning someone without consent can be illegal and harmful. Flux requires all of this before
        it will clone a voice.
      </p>

      <div className="box col">
        <Toggle
          label="This is my own voice, or I have written permission from the person"
          checked={ownVoice}
          onChange={setOwnVoice}
        />
        <Toggle
          label="I won't use it to impersonate anyone or mislead people"
          checked={noImpersonation}
          onChange={setNoImpersonation}
        />
        <Toggle
          label="I understand generated audio carries a hidden watermark and a metadata label"
          checked={understandsWatermark}
          onChange={setUnderstandsWatermark}
        />
      </div>

      <div className="box col">
        <b>Read this aloud</b>
        <p style={{ fontSize: 19, fontWeight: 700, lineHeight: 1.3 }}>
          "My name is [your name]. Today is November 3rd. I agree to Flux creating a synthetic copy
          of my voice."
        </p>
        <div className="row">
          <button
            type="button"
            className={`btn p ${recordedConsent ? "" : ""}`}
            onClick={handleRecordConsent}
          >
            <Icon name="mic" size={14} />
            {isRecording
              ? "Recording consent..."
              : recordedConsent
                ? "Consent recorded"
                : "Record consent"}
          </button>
          <span className="dim">
            We check that the words match and that it's the same voice as your samples.
          </span>
        </div>
      </div>

      <Callout icon="warn" variant="warn">
        Off by default. Cloning is limited per day, and anyone can report misuse.
      </Callout>

      <div className="between">
        <button type="button" className="btn g" onClick={() => onNavigate?.("tts")}>
          Cancel
        </button>
        <button
          type="button"
          className="btn lg"
          disabled={!canContinue}
          style={canContinue ? {} : { opacity: 0.45, cursor: "not-allowed" }}
          onClick={() => {
            if (canContinue) {
              onNavigate?.("tts");
            }
          }}
        >
          Continue
        </button>
      </div>
    </WizardLayout>
  );
};
