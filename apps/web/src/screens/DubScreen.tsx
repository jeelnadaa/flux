import React from "react";
import { ProjectLayout, Field, Select, Section, Icon } from "@flux/ui";

interface DubScreenProps {
  onNavigate?: (route: string) => void;
}

export const DubScreen: React.FC<DubScreenProps> = ({ onNavigate }) => {
  return (
    <ProjectLayout currentTab="aitools" onNavigate={onNavigate}>
      <div className="pg in">
        <div className="pgh">
          <div>
            <h1 className="h1">Say it in another language.</h1>
            <p className="lede">
              Flux translates the script, you check it, then it replaces the speech and keeps the
              music and room.
            </p>
          </div>
        </div>

        <div className="box g3">
          <Field label="From">
            <Select options={["English (detected)"]} />
          </Field>
          <Field label="To">
            <Select options={["Spanish", "Hindi", "French", "German", "Portuguese", "Japanese"]} />
          </Field>
          <Field label="Tone">
            <Select options={["Natural", "Formal", "Casual"]} />
          </Field>
        </div>

        <Section title="Voices">
          <div className="g2">
            <div className="box between">
              <b className="row" style={{ gap: 6, flexWrap: "nowrap" }}>
                <span className="dot" style={{ color: "var(--t4)" }} />
                Maya
              </b>
              <div style={{ width: "60%" }}>
                <Select options={["Ash (es)", "Marin (es)", "Reed (es)"]} />
              </div>
            </div>
            <div className="box between">
              <b className="row" style={{ gap: 6, flexWrap: "nowrap" }}>
                <span className="dot" style={{ color: "var(--t3)" }} />
                Tom
              </b>
              <div style={{ width: "60%" }}>
                <Select options={["Oren (es)", "Marin (es)", "Reed (es)"]} />
              </div>
            </div>
          </div>
        </Section>

        <Section title="Glossary">
          <div className="row">
            <button type="button" className="chip">
              Harbor Lights: keep
            </button>
            <button type="button" className="chip">
              Kessler: keep
            </button>
            <button type="button" className="chip">
              + Add term
            </button>
          </div>
        </Section>

        <div className="callout warn">
          <Icon name="cloud" />
          <div>
            <b>This needs the cloud.</b> Audio is sent encrypted, used only for this job, and
            deleted within 24 hours.{" "}
            <a
              className="act"
              href="#consent"
              data-go="consent"
              onClick={(e) => {
                e.preventDefault();
                onNavigate?.("consent");
              }}
            >
              See how it works
            </a>
          </div>
        </div>

        <div className="row">
          <button type="button" className="btn p lg" onClick={() => onNavigate?.("dubreview")}>
            Translate script
          </button>
          <span className="dim">
            Machine translation can be wrong. You'll review every line before anything is rendered.
          </span>
        </div>
      </div>
    </ProjectLayout>
  );
};
