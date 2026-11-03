import React from "react";
import { ProjectLayout, Icon } from "@flux/ui";

export const EmptyProjectScreen: React.FC = () => {
  return (
    <ProjectLayout currentTab="edit" projectName="Untitled project">
      <div
        className="pg in"
        style={{
          margin: "auto",
          alignItems: "center",
          textAlign: "center",
          maxWidth: "620px",
        }}
      >
        <h1 className="h1">Nothing here yet.</h1>
        <p className="lede">
          Add a recording to get started. Your original files are never changed.
        </p>

        <div className="row" style={{ justifyContent: "center" }}>
          <a className="btn p lg" href="#import" data-go="import">
            <Icon name="imp" />
            <span>Add a file</span>
          </a>
          <a className="btn lg" href="#record" data-go="record">
            <Icon name="mic" />
            <span>Record</span>
          </a>
          <a className="btn lg" href="#tts" data-go="tts">
            <Icon name="text" />
            <span>Generate speech</span>
          </a>
        </div>

        <div className="row dim" style={{ justifyContent: "center" }}>
          <span className="kbd">Ctrl O</span> open file <span className="kbd">R</span> record{" "}
          <span className="kbd">Ctrl K</span> all commands
        </div>
      </div>
    </ProjectLayout>
  );
};
