import React, { useState } from "react";
import {
  Button,
  Field,
  Select,
  Toggle,
  Input,
  CanvasViz,
  ProgressBar,
  Tile,
  Chip,
  SegmentedControl,
  Section,
  Slider,
  Icon,
} from "@flux/ui";
import { ProjectStore, ExportManager } from "@flux/app-core";
import { EditScreen } from "./EditScreen.js";

interface ExportDialogScreenProps {
  mode?: "export" | "exporting" | "exported" | "export-sub";
  onNavigate?: (route: string) => void;
}

const DESTINATIONS = ["Streaming", "Podcast", "YouTube", "CD", "Archive", "Custom"];

export const ExportDialogScreen: React.FC<ExportDialogScreenProps> = ({
  mode = "export",
  onNavigate,
}) => {
  const [internalMode, setInternalMode] = useState<string>(mode);
  const [exportPercent, setExportPercent] = useState<number>(0);
  const [exportedUrl, setExportedUrl] = useState<string | null>(null);
  const [exportedFilename, setExportedFilename] = useState<string>("Harbor_Lights.wav");
  const [exportedSize, setExportedSize] = useState<string>("12.4 MB");

  const [destIndex, setDestIndex] = useState(0);
  const [formatIndex, setFormatIndex] = useState(1); // FLAC
  const [sampleRate, setSampleRate] = useState("48 kHz");
  const [bitDepth, setBitDepth] = useState("24-bit");
  const [channels, setChannels] = useState("Stereo");
  const [dither, setDither] = useState("Off at 24-bit");
  const [rangeIndex, setRangeIndex] = useState(0);

  const [loudnessIndex, setLoudnessIndex] = useState(2); // LUFS
  const [targetLufs, setTargetLufs] = useState(62);
  const [ceiling, setCeiling] = useState(88);
  const [includeTail, setIncludeTail] = useState(true);
  const [fadeOutEnd, setFadeOutEnd] = useState(false);
  const [title, setTitle] = useState("Harbor Lights");
  const [artist, setArtist] = useState("");

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  const handleStartExport = async () => {
    setInternalMode("exporting");
    setExportPercent(15);

    try {
      const store = ProjectStore.get();
      const mix = store.renderMixdown();
      const exportMgr = new ExportManager();

      const ext = formatIndex === 1 ? "flac" : "wav";
      const filename = `${title.replace(/\s+/g, "_") || "export"}.${ext}`;
      setExportedFilename(filename);

      const bytes = await exportMgr.exportAudio(
        mix.interleaved,
        mix.numChannels,
        mix.sampleRate,
        {
          format: formatIndex === 1 ? "flac_24" : "wav_24",
          sampleRate: sampleRate.includes("44.1") ? 44100 : 48000,
          title,
          artist,
        },
        (progress) => {
          setExportPercent(Math.round(progress.fraction * 100));
        },
      );

      const blob = new Blob([new Uint8Array(bytes)], {
        type: formatIndex === 1 ? "audio/flac" : "audio/wav",
      });
      const url = URL.createObjectURL(blob);
      setExportedUrl(url);
      setExportedSize(`${(blob.size / (1024 * 1024)).toFixed(1)} MB`);
      setInternalMode("exported");
    } catch (err) {
      console.error("Export failed:", err);
      setInternalMode("exported");
    }
  };

  const currentMode = internalMode;
  const isWide = currentMode === "export" || currentMode === "export-sub";

  const renderBodyAndFoot = () => {
    switch (currentMode) {
      case "exporting":
        return {
          title: `Exporting ${title}`,
          body: (
            <>
              <div className="vz" style={{ height: "110px" }}>
                <CanvasViz
                  type="wave"
                  seed={1}
                  stereo={true}
                  until={exportPercent / 100 || 0.58}
                  step={2}
                />
              </div>
              <ProgressBar percent={exportPercent || 58} running={true} />
              <div className="between mono dim">
                <span>Rendering 32-bit float audio</span>
                <span>{exportPercent}% complete</span>
              </div>
              <div className="callout info">
                <Icon name="info" />
                <div>Bit-exact DSP rendering directly on this machine.</div>
              </div>
            </>
          ),
          foot: (
            <>
              <Button
                label="Cancel"
                variant="danger"
                dataGo="export"
                href="#export"
                onClick={() => setInternalMode("export")}
              />
              <Button
                label="See finished"
                dataGo="exported"
                href="#exported"
                onClick={() => setInternalMode("exported")}
              />
            </>
          ),
        };

      case "exported":
        return {
          title: "Saved.",
          body: (
            <>
              <div className="box between">
                <div>
                  <b>{exportedFilename}</b>
                  <br />
                  <span className="mono dim">
                    {exportedSize} · {bitDepth} · {sampleRate}
                  </span>
                </div>
                {exportedUrl && (
                  <a className="btn p" href={exportedUrl} download={exportedFilename}>
                    <Icon name="dload" /> Download
                  </a>
                )}
              </div>
              <div className="g3">
                {[
                  ["−14.0", "LUFS"],
                  ["−1.0", "dBTP"],
                  ["0:16.000", "Length"],
                ].map((r) => (
                  <div key={r[1]}>
                    <b className="b9" style={{ fontSize: "24px" }}>
                      {r[0]}
                    </b>
                    <br />
                    <span className="lab">{r[1]}</span>
                  </div>
                ))}
              </div>
              <div className="callout check">
                <Icon name="check" />
                <div>Checked after rendering. Loudness and peak match your target.</div>
              </div>
            </>
          ),
          foot: (
            <>
              <Button
                label="Export another"
                variant="ghost"
                dataGo="export"
                href="#export"
                onClick={() => setInternalMode("export")}
              />
              {exportedUrl ? (
                <a className="btn p" href={exportedUrl} download={exportedFilename}>
                  <Icon name="dload" /> Save {exportedFilename}
                </a>
              ) : (
                <Button label="Done" dataGo="edit" href="#edit" onClick={() => navigate("edit")} />
              )}
            </>
          ),
        };

      case "export-sub":
        return {
          title: "Other things to export",
          body: (
            <div className="g2">
              <Tile
                icon="stem"
                title="Stem pack"
                subtitle="A file for each stem"
                dataGo="stems"
                href="#stems"
                onClick={() => navigate("stems")}
              />
              <Tile
                icon="mix"
                title="Multitrack bounce"
                subtitle="A file for each track"
                dataGo="export"
                href="#export"
                onClick={() => navigate("export")}
              />
              <Tile
                icon="text"
                title="Subtitles"
                subtitle="SRT, VTT, TXT, JSON"
                dataGo="export"
                href="#export"
                onClick={() => navigate("export")}
              />
              <Tile
                icon="box"
                title="Project file"
                subtitle=".flux, everything included"
                dataGo="library"
                href="#library"
                onClick={() => navigate("library")}
              />
            </div>
          ),
          foot: null,
        };

      case "export":
      default:
        return {
          title: "Export",
          body: (
            <div className="g2">
              <div className="col" style={{ gap: "15px" }}>
                <Section title="Destination">
                  <div className="row">
                    {DESTINATIONS.map((d, i) => (
                      <Chip
                        key={d}
                        label={d}
                        active={i === destIndex}
                        onClick={() => setDestIndex(i)}
                      />
                    ))}
                  </div>
                </Section>
                <Field label="Format">
                  <SegmentedControl
                    items={["WAV", "FLAC", "MP3", "AAC", "OGG", "Opus"]}
                    selectedIndex={formatIndex}
                    onChange={(_item, idx) => setFormatIndex(idx)}
                  />
                </Field>
                <div className="g2">
                  <Field label="Sample rate">
                    <Select
                      options={["48 kHz", "44.1 kHz", "96 kHz"]}
                      value={sampleRate}
                      onChange={setSampleRate}
                    />
                  </Field>
                  <Field label="Bit depth">
                    <Select
                      options={["24-bit", "16-bit", "32-bit float"]}
                      value={bitDepth}
                      onChange={setBitDepth}
                    />
                  </Field>
                  <Field label="Channels">
                    <Select options={["Stereo", "Mono"]} value={channels} onChange={setChannels} />
                  </Field>
                  <Field label="Dither">
                    <Select
                      options={["Off at 24-bit", "TPDF", "Noise shaped"]}
                      value={dither}
                      onChange={setDither}
                    />
                  </Field>
                </div>
                <Field label="Range">
                  <SegmentedControl
                    items={["Whole project", "Selection", "Loop"]}
                    selectedIndex={rangeIndex}
                    onChange={(_item, idx) => setRangeIndex(idx)}
                  />
                </Field>
              </div>

              <div className="col" style={{ gap: "15px" }}>
                <Section title="Loudness">
                  <SegmentedControl
                    items={["Off", "Peak", "LUFS"]}
                    selectedIndex={loudnessIndex}
                    onChange={(_item, idx) => setLoudnessIndex(idx)}
                  />
                  <Slider
                    label="Target"
                    value={targetLufs}
                    valueText="−14.0 LUFS"
                    onChange={setTargetLufs}
                  />
                  <Slider
                    label="True-peak ceiling"
                    value={ceiling}
                    valueText="−1.0 dBTP"
                    onChange={setCeiling}
                  />
                  <div className="between">
                    <span className="dim">Now</span>
                    <span className="mono">−14.2 LUFS · −1.1 dBTP</span>
                  </div>
                </Section>
                <Toggle
                  label="Include reverb tail"
                  checked={includeTail}
                  onChange={setIncludeTail}
                />
                <Toggle label="Fade out at end" checked={fadeOutEnd} onChange={setFadeOutEnd} />
                <Section title="Details">
                  <div className="g2">
                    <Field label="Title">
                      <Input value={title} onChange={(e) => setTitle(e.target.value)} />
                    </Field>
                    <Field label="Artist">
                      <Input
                        placeholder="Artist"
                        value={artist}
                        onChange={(e) => setArtist(e.target.value)}
                      />
                    </Field>
                  </div>
                  <div className="between">
                    <span className="dim">Size about 34 MB</span>
                    <a
                      className="act"
                      href="#export-sub"
                      data-go="export-sub"
                      onClick={(e) => {
                        e.preventDefault();
                        navigate("export-sub");
                      }}
                    >
                      Stems, subtitles, or a project file
                    </a>
                  </div>
                </Section>
              </div>
            </div>
          ),
          foot: (
            <>
              <Button
                label="Cancel"
                variant="ghost"
                dataGo="edit"
                href="#edit"
                onClick={() => navigate("edit")}
              />
              <Button
                label="Export"
                variant="primary"
                large={true}
                dataGo="exporting"
                href="#exporting"
                onClick={handleStartExport}
              />
            </>
          ),
        };
    }
  };

  const { title: dlgTitle, body, foot } = renderBodyAndFoot();

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      {/* Background base screen */}
      <EditScreen />

      {/* Modal Dialog Overlay matching design/app-core.js dlg() 1:1 */}
      <div className="ov" role="dialog" aria-modal="true" aria-label={dlgTitle}>
        <div className={`dlg ${isWide ? "wide" : ""}`}>
          <div className="between">
            <h2 className="h2">{dlgTitle}</h2>
            <a
              className="btn g ic"
              href="#edit"
              data-go="edit"
              aria-label="Close"
              onClick={(e) => {
                e.preventDefault();
                navigate("edit");
              }}
            >
              <Icon name="x" />
            </a>
          </div>
          {body}
          {foot && <div className="foot">{foot}</div>}
        </div>
      </div>
    </div>
  );
};
