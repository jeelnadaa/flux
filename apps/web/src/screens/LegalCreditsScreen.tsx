import React from "react";
import { HelpLayout } from "./HelpLayout.js";

const CREDITS: readonly [string, string, string][] = [
  ["Bricolage Grotesque", "Default interface type", "SIL OFL 1.1"],
  ["Archivo", "Interface type option", "SIL OFL 1.1"],
  ["Schibsted Grotesk", "Interface type option", "SIL OFL 1.1"],
  ["Geist", "Interface type option", "SIL OFL 1.1"],
  ["Anybody", "Interface type option", "SIL OFL 1.1"],
  ["Satoshi", "Interface type option", "ITF Free Font License"],
  ["Geist Mono", "Numbers and labels", "SIL OFL 1.1"],
  ["[HRTF dataset]", "Spatial audio", "[VERIFY]"],
  ["[Stem model]", "Stems", "[VERIFY]"],
  ["[Speech model]", "Transcripts", "[VERIFY]"],
  ["LAME", "MP3 encoding", "LGPL"],
  ["libopus", "Opus", "BSD"],
];

interface LegalCreditsScreenProps {
  onNavigate?: (route: string) => void;
}

export const LegalCreditsScreen: React.FC<LegalCreditsScreenProps> = ({ onNavigate }) => {
  return (
    <HelpLayout currentTab="legal-credits" title="Attributions" onNavigate={onNavigate}>
      <div className="tw">
        <table>
          <thead>
            <tr>
              <th>Component</th>
              <th>Used for</th>
              <th>Licence</th>
            </tr>
          </thead>
          <tbody>
            {CREDITS.map(([name, usedFor, license]) => (
              <tr key={name}>
                <td className="b7">{name}</td>
                <td className="dim">{usedFor}</td>
                <td className="mono">{license}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </HelpLayout>
  );
};
