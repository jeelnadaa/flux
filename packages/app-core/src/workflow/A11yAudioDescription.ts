/**
 * Accessibility audio description engine for Flux per PLAN.md §8.5 & §Tests (Phase 8).
 * Generates natural language audio and waveform summaries for screen reader users
 * conforming to WCAG 2.2 AA.
 */

export interface WaveformSection {
  startSec: number;
  endSec: number;
  rmsDbfs: number;
  peakDbfs: number;
  label: "silent" | "quiet" | "moderate" | "loud" | "climax";
}

export class A11yAudioDescription {
  /**
   * Analyzes an audio buffer and returns a rich textual description of the waveform's
   * energy envelope, loud and quiet sections, and peak levels for screen reader users.
   */
  public static describeWaveform(samples: Float32Array, sampleRate: number): string {
    const totalDurationSec = samples.length / sampleRate;
    if (totalDurationSec === 0 || samples.length === 0) {
      return "Empty audio clip with no audible content.";
    }

    const windowSec = Math.max(2.0, Math.min(10.0, totalDurationSec / 10));
    const windowSamples = Math.floor(windowSec * sampleRate);
    const sections: WaveformSection[] = [];

    let overallPeak = 0.0;

    for (let start = 0; start < samples.length; start += windowSamples) {
      const end = Math.min(samples.length, start + windowSamples);
      let sumSq = 0.0;
      let peak = 0.0;
      const count = end - start;

      for (let i = start; i < end; i++) {
        const absVal = Math.abs(samples[i]);
        if (absVal > peak) peak = absVal;
        sumSq += absVal * absVal;
      }

      if (peak > overallPeak) overallPeak = peak;

      const rms = Math.sqrt(sumSq / Math.max(1, count));
      const rmsDbfs = rms > 1e-6 ? 20 * Math.log10(rms) : -100.0;
      const peakDbfs = peak > 1e-6 ? 20 * Math.log10(peak) : -100.0;

      let label: WaveformSection["label"] = "moderate";
      if (rmsDbfs < -45.0) {
        label = "silent";
      } else if (rmsDbfs < -24.0) {
        label = "quiet";
      } else if (rmsDbfs < -12.0) {
        label = "moderate";
      } else if (peakDbfs > -3.0) {
        label = "climax";
      } else {
        label = "loud";
      }

      sections.push({
        startSec: start / sampleRate,
        endSec: end / sampleRate,
        rmsDbfs,
        peakDbfs,
        label,
      });
    }

    const totalMinutes = Math.floor(totalDurationSec / 60);
    const totalSeconds = Math.round(totalDurationSec % 60);
    const timeStr = totalMinutes > 0 ? `${totalMinutes}m ${totalSeconds}s` : `${totalSeconds}s`;

    const overallPeakDbfs = overallPeak > 1e-6 ? (20 * Math.log10(overallPeak)).toFixed(1) : "-inf";

    // Summarize narrative arc
    const parts: string[] = [
      `Audio track duration ${timeStr}, peak level ${overallPeakDbfs} dBFS.`,
    ];

    if (sections.length <= 2) {
      const first = sections[0];
      parts.push(`Overall volume is ${first.label}.`);
    } else {
      const intro = sections[0];
      parts.push(`Begins ${intro.label} for ${intro.endSec.toFixed(0)} seconds.`);

      const loudSection = sections.find((s) => s.label === "loud" || s.label === "climax");
      if (loudSection) {
        const startMin = Math.floor(loudSection.startSec / 60);
        const startSec = Math.round(loudSection.startSec % 60);
        const startFormatted =
          startMin > 0
            ? `${startMin}:${startSec.toString().padStart(2, "0")}`
            : `0:${startSec.toString().padStart(2, "0")}`;
        parts.push(`Peak energy occurs around ${startFormatted} (${loudSection.label}).`);
      }

      const quietSection = sections.find((s) => s.label === "quiet" || s.label === "silent");
      if (quietSection && quietSection !== sections[0]) {
        const startSec = Math.round(quietSection.startSec);
        parts.push(`Quiet passage around ${startSec} seconds.`);
      }
    }

    return parts.join(" ");
  }
}
