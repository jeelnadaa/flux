import React from "react";
import { HelpLayout } from "./HelpLayout.js";

interface LegalPrivacyScreenProps {
  onNavigate?: (route: string) => void;
}

export const LegalPrivacyScreen: React.FC<LegalPrivacyScreenProps> = ({ onNavigate }) => {
  return (
    <HelpLayout currentTab="legal" title="Privacy" onNavigate={onNavigate}>
      <div className="legal">
        <p>[DRAFT: needs legal review before launch.] This page is laid out for the final text.</p>
        <h3>What stays on your device</h3>
        <p>
          Your audio, projects, transcripts and AI results are stored and processed on your device.
        </p>
        <h3>What leaves your device</h3>
        <p>
          Nothing, unless you turn on a cloud feature for a specific job. Cloud audio is encrypted
          in transit and deleted within 24 hours.
        </p>
        <h3>Your rights</h3>
        <p>You can delete all cloud data from Settings at any time.</p>
      </div>
    </HelpLayout>
  );
};
