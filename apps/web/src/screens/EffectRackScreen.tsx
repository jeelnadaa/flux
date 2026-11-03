import React, { useState } from "react";
import {
  ProjectLayout,
  SegmentedControl,
  Button,
  Icon,
  Slider,
  Section,
  Pill,
  ProgressBar,
} from "@flux/ui";
import { EffectChain, type EffectInstance } from "@flux/app-core";

export const EffectRackScreen: React.FC = () => {
  const [chain, setChain] = useState<EffectChain>(() => EffectChain.createDefaultVocalsChain());
  const [targetIndex, setTargetIndex] = useState(1); // 'Track' by default
  const [compareSlotIndex, setCompareSlotIndex] = useState(0); // 'A' by default
  const [isHoldingBefore, setIsHoldingBefore] = useState(false);
  const [macroValue, setMacroValue] = useState(62);

  const handleMixChange = (effectId: string, val: number) => {
    const updated = chain.clone();
    updated.setMix(effectId, val);
    setChain(updated);
  };

  const handleToggleSolo = (effectId: string) => {
    const updated = chain.clone();
    updated.toggleSolo(effectId);
    setChain(updated);
  };

  const handleToggleBypass = (effectId: string) => {
    const updated = chain.clone();
    updated.toggleBypass(effectId);
    setChain(updated);
  };

  const handleMacroChange = (val: number) => {
    setMacroValue(val);
    const updated = chain.clone();
    updated.setMacroValue("Presence", val);
    setChain(updated);
  };

  const handleCompareSlotChange = (_item: string, index: number) => {
    setCompareSlotIndex(index);
    const slot = (["A", "B", "C"] as const)[index];
    const updated = chain.clone();
    updated.switchCompareSlot(slot);
    setChain(updated);
  };

  const cpuPercent = chain.estimateCpuPercent();

  return (
    <ProjectLayout currentTab="fxbrowser">
      <div className="split">
        <div className="a">
          <div className="tlbar">
            <b style={{ fontSize: "15px", fontWeight: 800 }}>Vocals chain</b>
            <SegmentedControl
              items={["Clip", "Track", "Master"]}
              selectedIndex={targetIndex}
              onChange={(_item, idx) => setTargetIndex(idx)}
            />
            <span className="sp" style={{ flex: 1 }} />
            <SegmentedControl
              items={["A", "B", "C"]}
              selectedIndex={compareSlotIndex}
              onChange={handleCompareSlotChange}
            />
            <Button label="Save preset" icon="tag" />
            <Button
              label="Add effect"
              variant="primary"
              icon="plus"
              dataGo="fxbrowser"
              href="#fxbrowser"
            />
          </div>

          <div
            className="in"
            style={{
              padding: "14px",
              display: "flex",
              flexDirection: "column",
              gap: "9px",
            }}
          >
            {chain.effects.map((item: EffectInstance) => (
              <div
                key={item.id}
                className="box row"
                style={{ flexWrap: "nowrap", padding: "10px 12px" }}
              >
                <span className="faint" style={{ cursor: "grab" }} aria-hidden="true">
                  <Icon name="list" size={17} />
                </span>
                <span className="dot" style={{ color: `var(${item.colorVar})` }} />
                <a
                  href={`#${item.route}`}
                  data-go={item.route}
                  style={{ textDecoration: "none", minWidth: 0, flex: 1 }}
                >
                  <b
                    className="b7"
                    style={{
                      display: "block",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {item.name}
                  </b>
                  <span className="lab hide-s">{item.subtitle}</span>
                </a>
                <div style={{ width: "110px" }} className="hide-s">
                  <Slider
                    label="Mix"
                    value={item.mix}
                    valueText={`${item.mix}%`}
                    onChange={(val) => handleMixChange(item.id, val)}
                  />
                </div>
                <button
                  className={`chip ${item.soloed ? "on" : ""}`}
                  data-toggle=""
                  type="button"
                  onClick={() => handleToggleSolo(item.id)}
                >
                  Solo
                </button>
                <button
                  className="tg"
                  style={{ width: "auto" }}
                  type="button"
                  aria-pressed={!item.bypassed}
                  aria-label={`Enable ${item.name}`}
                  onClick={() => handleToggleBypass(item.id)}
                >
                  <i />
                </button>
              </div>
            ))}
          </div>

          <div className="box" style={{ margin: "0 14px 14px" }}>
            <div className="between">
              <b>Compare</b>
              <span className="dim">Levels are matched so louder doesn't win</span>
            </div>
            <div className="row" style={{ marginTop: "9px" }}>
              <button
                className={`btn lg hold p ${isHoldingBefore ? "on" : ""}`}
                data-hold=""
                type="button"
                style={{ flex: 1 }}
                onPointerDown={() => setIsHoldingBefore(true)}
                onPointerUp={() => setIsHoldingBefore(false)}
                onPointerLeave={() => setIsHoldingBefore(false)}
              >
                Hold for before
              </button>
            </div>
          </div>
        </div>

        <aside className="b in">
          <h3 className="h3">Macro</h3>
          <p className="dim">One slider that moves several controls at once.</p>
          <Slider
            label="Presence"
            value={macroValue}
            valueText={`${macroValue}%`}
            big
            onChange={handleMacroChange}
          />
          <Section title="This macro moves">
            <div className="lst">
              <div>
                <div className="ti">
                  <b>EQ band 4 gain</b>
                  <span>0 to +5 dB</span>
                </div>
              </div>
              <div>
                <div className="ti">
                  <b>De-esser amount</b>
                  <span>−1 to −5 dB</span>
                </div>
              </div>
              <div>
                <div className="ti">
                  <b>Compressor mix</b>
                  <span>60% to 100%</span>
                </div>
              </div>
            </div>
            <Button label="Map a control" icon="plus" />
          </Section>
          <Section title="CPU">
            <div className="between">
              <span className="mono">{cpuPercent}%</span>
              <Pill label="Real time" variant="ok" />
            </div>
            <ProgressBar percent={cpuPercent} />
          </Section>
        </aside>
      </div>
    </ProjectLayout>
  );
};
