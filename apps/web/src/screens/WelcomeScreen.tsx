import React from "react";
import { WizardLayout, Slider, CanvasViz, Button } from "@flux/ui";

export const WelcomeScreen: React.FC = () => {
  return (
    <WizardLayout step={1} totalSteps={4} watermarkGo="welcome">
      <h1 className="bigh">Hello.</h1>
      <p className="lede">
        How do you want to work? You can switch any time. Nothing you make is lost when you do.
      </p>
      <div className="g2">
        <a className="tile" href="#home" data-go="home">
          <div className="thumb" style={{ height: "96px", padding: "14px" }}>
            <Slider label="Amount" defaultValue={62} valueText="62%" big />
          </div>
          <b>Simple</b>
          <span>Pick what you want done. One slider, then export.</span>
        </a>
        <a className="tile" href="#edit" data-go="edit">
          <div className="thumb" style={{ height: "96px" }}>
            <CanvasViz type="wave" seed={3} stereo />
          </div>
          <b>Pro</b>
          <span>Timeline, automation, spectral editing, every parameter.</span>
        </a>
      </div>
      <div className="between">
        <a className="dim" href="#home" data-go="home">
          Skip setup
        </a>
        <Button variant="primary" large label="Continue" dataGo="home" href="#home" />
      </div>
    </WizardLayout>
  );
};
