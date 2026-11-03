import React from "react";
import { HelpLayout } from "./HelpLayout.js";

interface LegalTermsScreenProps {
  onNavigate?: (route: string) => void;
}

export const LegalTermsScreen: React.FC<LegalTermsScreenProps> = ({ onNavigate }) => {
  return (
    <HelpLayout currentTab="legal-terms" title="Terms" onNavigate={onNavigate}>
      <div className="legal">
        <p>[DRAFT: needs legal review before launch.]</p>
        <h3>Your content</h3>
        <p>You keep your rights. Only process audio you have the right to use.</p>
        <h3>Voices</h3>
        <p>
          Don't clone or imitate someone without their consent, or use Flux to mislead people.
          Generated speech is labeled.
        </p>
      </div>
    </HelpLayout>
  );
};
