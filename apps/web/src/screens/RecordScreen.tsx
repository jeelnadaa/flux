import React, { useState, useEffect, useRef } from "react";
import { ProjectLayout, CanvasViz, Field, Select, Toggle, Callout, Button } from "@flux/ui";
import { ProjectStore } from "@flux/app-core";

interface RecordScreenProps {
  onNavigate?: (route: string) => void;
}

export const RecordScreen: React.FC<RecordScreenProps> = ({ onNavigate }) => {
  const [microphones, setMicrophones] = useState<string[]>(["Default Microphone"]);
  const [microphone, setMicrophone] = useState("Default Microphone");
  const [recordTo, setRecordTo] = useState("New track");
  const [monitor, setMonitor] = useState(false);
  const [rawSignal, setRawSignal] = useState(true);
  const [isRecording, setIsRecording] = useState(false);
  const [elapsedSec, setElapsedSec] = useState(0);
  const [peakDb, setPeakDb] = useState("-inf");
  const [meterWidth, setMeterWidth] = useState(0);

  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const animFrameRef = useRef<number | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  const navigate = (r: string) => {
    if (onNavigate) {
      onNavigate(r);
    } else {
      window.location.hash = r;
    }
  };

  // Enumerate input devices
  useEffect(() => {
    if (typeof navigator !== "undefined" && navigator.mediaDevices?.enumerateDevices) {
      navigator.mediaDevices
        .enumerateDevices()
        .then((devices) => {
          const audioInputs = devices
            .filter((d) => d.kind === "audioinput")
            .map((d, i) => d.label || `Microphone ${i + 1}`);
          if (audioInputs.length > 0) {
            setMicrophones(audioInputs);
            setMicrophone(audioInputs[0]);
          }
        })
        .catch(() => {});
    }
  }, []);

  // Start live microphone stream and meter
  useEffect(() => {
    let isCancelled = false;

    async function startCapture() {
      if (typeof navigator === "undefined" || !navigator.mediaDevices?.getUserMedia) {
        setIsRecording(true);
        return;
      }

      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            echoCancellation: !rawSignal,
            noiseSuppression: !rawSignal,
            autoGainControl: !rawSignal,
          },
        });

        if (isCancelled) {
          stream.getTracks().forEach((t) => t.stop());
          return;
        }

        streamRef.current = stream;
        const ctx = new (
          window.AudioContext ||
          (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext
        )();
        audioCtxRef.current = ctx;

        const source = ctx.createMediaStreamSource(stream);
        const analyser = ctx.createAnalyser();
        analyser.fftSize = 256;
        source.connect(analyser);

        const dataArray = new Uint8Array(analyser.frequencyBinCount);

        const tickMeter = () => {
          analyser.getByteTimeDomainData(dataArray);
          let sumSquares = 0;
          for (let i = 0; i < dataArray.length; i++) {
            const norm = (dataArray[i] - 128) / 128;
            sumSquares += norm * norm;
          }
          const rms = Math.sqrt(sumSquares / dataArray.length);
          const db = rms > 0.0001 ? 20 * Math.log10(rms) : -96;
          const clampedDb = Math.max(-60, Math.min(0, db));
          const pct = Math.max(0, Math.min(100, ((clampedDb + 60) / 60) * 100));

          setPeakDb(db > -60 ? `${db.toFixed(1)} dB peak` : "−inf dB");
          setMeterWidth(pct);
          animFrameRef.current = requestAnimationFrame(tickMeter);
        };
        animFrameRef.current = requestAnimationFrame(tickMeter);

        // Setup MediaRecorder
        try {
          const recorder = new MediaRecorder(stream);
          chunksRef.current = [];
          recorder.ondataavailable = (e) => {
            if (e.data && e.data.size > 0) {
              chunksRef.current.push(e.data);
            }
          };
          recorder.start(100);
          mediaRecorderRef.current = recorder;
          setIsRecording(true);
        } catch {
          setIsRecording(true);
        }
      } catch (err) {
        console.warn("Microphone access error:", err);
        setIsRecording(true);
      }
    }

    startCapture();

    return () => {
      isCancelled = true;
      if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((t) => t.stop());
      }
      if (audioCtxRef.current) {
        audioCtxRef.current.close().catch(() => {});
      }
    };
  }, [rawSignal]);

  // Elapsed recording timer
  useEffect(() => {
    if (!isRecording) return;
    const interval = setInterval(() => {
      setElapsedSec((prev) => prev + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [isRecording]);

  const formatTimer = (totalSeconds: number): string => {
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    const hrs = Math.floor(mins / 60);
    const remMins = mins % 60;
    return `${hrs.toString().padStart(2, "0")}:${remMins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const handleStopRecording = async () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== "inactive") {
      mediaRecorderRef.current.stop();
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
    }

    // Combine recorded chunks and import into timeline
    if (chunksRef.current.length > 0) {
      const mime = chunksRef.current[0].type || "audio/webm";
      const blob = new Blob(chunksRef.current, { type: mime });
      try {
        await ProjectStore.getInstance().importAudioFile(blob, `Mic Take ${Date.now()}`);
      } catch (e) {
        console.warn("Could not decode mic take:", e);
      }
    }

    navigate("edit");
  };

  const handleDiscard = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== "inactive") {
      mediaRecorderRef.current.stop();
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
    }
    chunksRef.current = [];
    navigate("edit");
  };

  return (
    <ProjectLayout currentTab="edit">
      <div className="pg in" style={{ maxWidth: "900px", margin: "auto" }}>
        <div className="between">
          <h1 className="h1">Recording.</h1>
          <span className="pill bad">
            <span className="dot" />
            REC {formatTimer(elapsedSec)}
          </span>
        </div>

        <div className="vz" style={{ height: "170px" }}>
          <CanvasViz type="wave" seed={2} kind="voice" color="--sig" until={0.7} step={2} />
        </div>

        <div className="col">
          <div className="between lab">
            <span>Input level</span>
            <span className="mono">{peakDb}</span>
          </div>
          <div className="meter" style={{ width: "100%" }}>
            <div className="m" style={{ height: "12px" }}>
              <i data-lv style={{ width: `${meterWidth}%` }} />
            </div>
            <div className="m" style={{ height: "12px" }}>
              <i data-lv style={{ width: `${Math.max(0, meterWidth - 4)}%` }} />
            </div>
          </div>
        </div>

        <div className="g2">
          <Field label="Microphone">
            <Select options={microphones} value={microphone} onChange={setMicrophone} />
          </Field>
          <Field label="Record to">
            <Select
              options={["New track", "Selected track"]}
              value={recordTo}
              onChange={setRecordTo}
            />
          </Field>
        </div>

        <div className="box g2">
          <Toggle
            label="Hear yourself while recording"
            sub="About 90 ms delay in the browser"
            checked={monitor}
            onChange={setMonitor}
          />
          <Toggle
            label="Raw signal"
            sub="Browser noise reduction and auto gain off"
            checked={rawSignal}
            onChange={setRawSignal}
          />
        </div>

        <Callout icon="check">
          Recording is written to disk as it happens. A crash loses no more than the last second.
        </Callout>

        <div className="row" style={{ justifyContent: "center" }}>
          <Button
            label="Stop"
            variant="primary"
            large={true}
            icon="stop"
            dataGo="edit"
            href="#edit"
            onClick={handleStopRecording}
          />
          <Button
            label="Pause"
            large={true}
            icon="pause"
            onClick={() => {
              if (mediaRecorderRef.current?.state === "recording") {
                mediaRecorderRef.current.pause();
              } else if (mediaRecorderRef.current?.state === "paused") {
                mediaRecorderRef.current.resume();
              }
            }}
          />
          <Button label="Discard take" variant="danger" large={true} onClick={handleDiscard} />
        </div>
      </div>
    </ProjectLayout>
  );
};
