import type { SerializedCommand } from "./Command.js";

/**
 * Standard IEEE 802.3 CRC32 lookup table
 */
const CRC_TABLE = new Uint32Array(256);
for (let i = 0; i < 256; i++) {
  let c = i;
  for (let j = 0; j < 8; j++) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  CRC_TABLE[i] = c >>> 0;
}

export function computeCrc32(data: string | Uint8Array): number {
  let crc = 0xffffffff;
  if (typeof data === "string") {
    for (let i = 0; i < data.length; i++) {
      const code = data.charCodeAt(i);
      if (code < 128) {
        crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ code) & 0xff];
      } else {
        // Multi-byte UTF-8
        const encoded = new TextEncoder().encode(data[i]);
        for (let b = 0; b < encoded.length; b++) {
          crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ encoded[b]) & 0xff];
        }
      }
    }
  } else {
    for (let i = 0; i < data.length; i++) {
      crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ data[i]) & 0xff];
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

export interface JournalRecord {
  seq: number;
  timestamp: number;
  cmd?: SerializedCommand;
  checkpoint?: unknown;
  exportState?: { name: string; progress: number };
}

export interface RecoveryReport {
  recoveredEditsCount: number;
  lastEditName: string;
  lastEditTime: number;
  interruptedExport?: { name: string; progress: number };
  commands: SerializedCommand[];
  hasCorruptedTrailingData: boolean;
}

/**
 * Append-only crash-safe journal with CRC32 verification
 */
export class ProjectJournal {
  private seq = 0;
  private storageKey: string;
  private memoryBuffer: string[] = [];

  constructor(readonly projectName = "Harbor Lights") {
    this.storageKey = `flux_journal_${encodeURIComponent(projectName)}`;
  }

  /**
   * Append a committed command to the journal with CRC32 checksum
   */
  async appendCommand(cmd: SerializedCommand): Promise<void> {
    this.seq++;
    const record: JournalRecord = {
      seq: this.seq,
      timestamp: Date.now(),
      cmd,
    };
    await this.writeRecord(record);
  }

  /**
   * Record in-flight export status (resumed on crash)
   */
  async recordExportState(name: string, progress: number): Promise<void> {
    this.seq++;
    const record: JournalRecord = {
      seq: this.seq,
      timestamp: Date.now(),
      exportState: { name, progress },
    };
    await this.writeRecord(record);
  }

  private async writeRecord(record: JournalRecord): Promise<void> {
    const payload = JSON.stringify(record);
    const crc = computeCrc32(payload);
    const line = `${payload}|${crc}\n`;

    this.memoryBuffer.push(line);

    if (typeof localStorage !== "undefined") {
      try {
        const existing = localStorage.getItem(this.storageKey) || "";
        localStorage.setItem(this.storageKey, existing + line);
      } catch {
        // Fallback or quota handling
      }
    }
  }

  /**
   * Chaos test helper: injects raw uncommitted/truncated data to simulate crash mid-write
   */
  injectRawData(raw: string): void {
    this.memoryBuffer.push(raw);
    if (typeof localStorage !== "undefined") {
      const existing = localStorage.getItem(this.storageKey) || "";
      localStorage.setItem(this.storageKey, existing + raw);
    }
  }

  /**
   * Parse journal, discard partial/corrupted trailing writes, and produce recovery report
   */
  async recover(): Promise<RecoveryReport> {
    let rawContent = this.memoryBuffer.join("");
    if (typeof localStorage !== "undefined") {
      const stored = localStorage.getItem(this.storageKey);
      if (stored) {
        rawContent = stored;
      }
    }

    const lines = rawContent.split("\n").filter((l) => l.trim().length > 0);
    const commands: SerializedCommand[] = [];
    let lastEditName = "None";
    let lastEditTime = Date.now();
    let interruptedExport: { name: string; progress: number } | undefined;
    let hasCorruptedTrailingData = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const sepIdx = line.lastIndexOf("|");
      if (sepIdx === -1) {
        hasCorruptedTrailingData = true;
        break; // Corrupted or truncated line mid-write
      }

      const payload = line.slice(0, sepIdx);
      const expectedCrc = Number(line.slice(sepIdx + 1));

      if (isNaN(expectedCrc)) {
        hasCorruptedTrailingData = true;
        break;
      }

      const actualCrc = computeCrc32(payload);
      if (actualCrc !== expectedCrc) {
        hasCorruptedTrailingData = true;
        break; // Checksum mismatch
      }

      try {
        const record = JSON.parse(payload) as JournalRecord;
        if (record.cmd) {
          commands.push(record.cmd);
          lastEditName = record.cmd.name;
          lastEditTime = record.timestamp;
        }
        if (record.exportState) {
          interruptedExport = record.exportState;
        }
      } catch {
        hasCorruptedTrailingData = true;
        break;
      }
    }

    return {
      recoveredEditsCount: commands.length,
      lastEditName,
      lastEditTime,
      interruptedExport,
      commands,
      hasCorruptedTrailingData,
    };
  }

  /**
   * Reset journal
   */
  async clear(): Promise<void> {
    this.seq = 0;
    this.memoryBuffer = [];
    if (typeof localStorage !== "undefined") {
      localStorage.removeItem(this.storageKey);
    }
  }
}
