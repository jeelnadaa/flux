import React from "react";
import { WizardLayout, Icon, Button } from "@flux/ui";

interface StorageAskScreenProps {
  onNavigate?: (route: string) => void;
}

export const StorageAskScreen: React.FC<StorageAskScreenProps> = ({ onNavigate }) => {
  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  return (
    <WizardLayout step={4} totalSteps={4}>
      <h1 className="h1">Where should your projects live?</h1>
      <p className="lede">
        Browsers can clear their own storage when space runs low. A real folder means your work
        can't disappear.
      </p>

      <div className="g2">
        <button className="tile" type="button" style={{ padding: "26px", textAlign: "left" }}>
          <span className="act">
            <Icon name="disk" size={32} />
          </span>
          <b style={{ fontSize: "18px" }}>Pick a folder on your computer</b>
          <span>Projects save directly to files you can back up or copy. Recommended.</span>
        </button>

        <button className="tile" type="button" style={{ padding: "26px", textAlign: "left" }}>
          <span className="act">
            <Icon name="globe" size={32} />
          </span>
          <b style={{ fontSize: "18px" }}>Keep them in this browser</b>
          <span>Nothing to pick now. You can move them to a folder later.</span>
        </button>
      </div>

      <div className="between">
        <a
          className="btn g"
          href="#hrtf"
          data-go="hrtf"
          onClick={(e) => {
            e.preventDefault();
            navigate("hrtf");
          }}
        >
          Back
        </a>
        <Button
          label="Start editing"
          variant="primary"
          large={true}
          dataGo="home"
          href="#home"
          onClick={(e) => {
            e.preventDefault();
            navigate("home");
          }}
        />
      </div>
    </WizardLayout>
  );
};
