import React from "react";
import { SettingsLayout, Callout, Button, Pill } from "@flux/ui";

export const AccountSettingsScreen: React.FC = () => {
  return (
    <SettingsLayout currentTab="account" title="Account">
      <Callout icon="info">
        You don't need an account. Everything on your device works without one. Sign in only for
        cloud features or a paid plan.
      </Callout>

      <div className="box col">
        <b>Not signed in</b>
        <div className="row">
          <Button label="Sign in" variant="primary" />
          <Button label="Create account" />
        </div>
      </div>

      <div className="box col">
        <div className="between">
          <b>Plan</b>
          <Pill label="Free" />
        </div>
        <span className="dim">Pricing isn't set yet. [PLAN DETAILS]</span>
      </div>
    </SettingsLayout>
  );
};
