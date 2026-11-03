import type { ProjectState, Clip, Track } from "../editing/EditingModel.js";
import { TimelineEditor } from "../editing/EditingModel.js";

export interface SerializedCommand {
  id: string;
  type: string;
  name: string;
  timestamp: number;
  params: Record<string, unknown>;
}

export interface Command<TState = ProjectState> {
  readonly id: string;
  readonly type: string;
  readonly name: string;
  readonly timestamp: number;
  do(state: TState): TState;
  undo(state: TState): TState;
  merge?(prev: Command<TState>): Command<TState> | null;
  serialize(): SerializedCommand;
}

export interface HistoryNode {
  id: string;
  commandId: string;
  name: string;
  timestamp: number;
  parentId: string | null;
  childrenIds: string[];
  branchName?: string;
  stateHash: string;
  isSnapshot?: boolean;
  snapshotName?: string;
}

export interface ProjectSnapshot {
  id: string;
  name: string;
  timestamp: number;
  stateHash: string;
  state: ProjectState;
}

/**
 * Fast deterministic canonical hash of ProjectState
 */
export function computeProjectHash(state: ProjectState): string {
  const canonical = {
    sampleRate: state.sampleRate,
    durationSeconds: Number(state.durationSeconds.toFixed(6)),
    tracks: state.tracks.map((t) => ({
      id: t.id,
      name: t.name,
      gainDb: Number(t.gainDb.toFixed(3)),
      pan: Number(t.pan.toFixed(3)),
      muted: t.muted,
      soloed: t.soloed,
      lanes: t.automationLanes.map((l) => ({
        id: l.id,
        param: l.parameter,
        pts: l.points.map((p) => [Number(p.timeSeconds.toFixed(6)), Number(p.value.toFixed(4))]),
      })),
    })),
    clips: state.clips
      .slice()
      .sort((a, b) => a.id.localeCompare(b.id))
      .map((c) => ({
        id: c.id,
        trackId: c.trackId,
        start: Number(c.startSeconds.toFixed(6)),
        duration: Number(c.durationSeconds.toFixed(6)),
        offset: Number(c.sourceOffsetSeconds.toFixed(6)),
        gain: Number(c.gainDb.toFixed(3)),
        reversed: c.reversed,
        fIn: Number(c.fadeInDurationSeconds.toFixed(6)),
        fInC: c.fadeInCurve,
        fOut: Number(c.fadeOutDurationSeconds.toFixed(6)),
        fOutC: c.fadeOutCurve,
      })),
    master: {
      gainDb: Number(state.master.gainDb.toFixed(3)),
      pan: Number(state.master.pan.toFixed(3)),
      muted: state.master.muted,
    },
  };

  const str = JSON.stringify(canonical);
  let hash = 0x811c9dc5;
  for (let i = 0; i < str.length; i++) {
    hash ^= str.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

let nextCmdId = 1;
function genId(prefix = "cmd"): string {
  return `${prefix}_${Date.now()}_${nextCmdId++}`;
}

/**
 * Concrete Command: Move Clip (with drag coalescing)
 */
export class MoveClipCommand implements Command<ProjectState> {
  readonly id: string;
  readonly type = "clip.move";
  readonly name: string;
  readonly timestamp: number;

  constructor(
    readonly clipId: string,
    readonly prevTrackId: string,
    readonly prevStartSeconds: number,
    readonly newTrackId: string,
    readonly newStartSeconds: number,
    clipName?: string,
  ) {
    this.id = genId("mv");
    this.name = `Move "${clipName || clipId}" to ${newStartSeconds.toFixed(3)}s`;
    this.timestamp = Date.now();
  }

  do(state: ProjectState): ProjectState {
    const editor = new TimelineEditor(state);
    editor.moveClip(this.clipId, this.newStartSeconds, this.newTrackId);
    return editor.getState();
  }

  undo(state: ProjectState): ProjectState {
    const editor = new TimelineEditor(state);
    editor.moveClip(this.clipId, this.prevStartSeconds, this.prevTrackId);
    return editor.getState();
  }

  merge(prev: Command<ProjectState>): Command<ProjectState> | null {
    if (prev instanceof MoveClipCommand && prev.clipId === this.clipId) {
      if (this.timestamp - prev.timestamp < 1000) {
        return new MoveClipCommand(
          this.clipId,
          prev.prevTrackId,
          prev.prevStartSeconds,
          this.newTrackId,
          this.newStartSeconds,
        );
      }
    }
    return null;
  }

  serialize(): SerializedCommand {
    return {
      id: this.id,
      type: this.type,
      name: this.name,
      timestamp: this.timestamp,
      params: {
        clipId: this.clipId,
        prevTrackId: this.prevTrackId,
        prevStartSeconds: this.prevStartSeconds,
        newTrackId: this.newTrackId,
        newStartSeconds: this.newStartSeconds,
      },
    };
  }
}

/**
 * Concrete Command: Split Clip
 */
export class SplitClipCommand implements Command<ProjectState> {
  readonly id: string;
  readonly type = "clip.split";
  readonly name: string;
  readonly timestamp: number;
  private createdClipId: string | null = null;

  constructor(
    readonly clipId: string,
    readonly splitTimelineSeconds: number,
    clipName?: string,
  ) {
    this.id = genId("splt");
    this.name = `Split "${clipName || clipId}" at ${splitTimelineSeconds.toFixed(3)}s`;
    this.timestamp = Date.now();
  }

  do(state: ProjectState): ProjectState {
    const editor = new TimelineEditor(state);
    const splitResult = editor.splitClip(this.clipId, this.splitTimelineSeconds);
    const nextState = editor.getState();
    if (splitResult && splitResult[1]) {
      this.createdClipId = splitResult[1].id;
    }
    return nextState;
  }

  undo(state: ProjectState): ProjectState {
    const targetClip = state.clips.find((c) => c.id === this.clipId);
    const createdClip = this.createdClipId
      ? state.clips.find((c) => c.id === this.createdClipId)
      : null;

    if (!targetClip) return state;

    const originalDuration = targetClip.durationSeconds + (createdClip?.durationSeconds || 0);

    return {
      ...state,
      clips: state.clips
        .filter((c: Clip) => c.id !== this.createdClipId)
        .map((c: Clip) => {
          if (c.id === this.clipId) {
            return {
              ...c,
              durationSeconds: originalDuration,
            };
          }
          return c;
        }),
    };
  }

  serialize(): SerializedCommand {
    return {
      id: this.id,
      type: this.type,
      name: this.name,
      timestamp: this.timestamp,
      params: {
        clipId: this.clipId,
        splitTimelineSeconds: this.splitTimelineSeconds,
      },
    };
  }
}

/**
 * Concrete Command: Delete Clip (with optional ripple)
 */
export class DeleteClipCommand implements Command<ProjectState> {
  readonly id: string;
  readonly type = "clip.delete";
  readonly name: string;
  readonly timestamp: number;
  private originalClips: Clip[] = [];

  constructor(
    readonly clipId: string,
    readonly ripple = false,
    clipName?: string,
  ) {
    this.id = genId("del");
    this.name = `Delete "${clipName || clipId}"${ripple ? " with ripple" : ""}`;
    this.timestamp = Date.now();
  }

  do(state: ProjectState): ProjectState {
    this.originalClips = state.clips;
    const editor = new TimelineEditor(state);
    editor.deleteClip(this.clipId, this.ripple);
    return editor.getState();
  }

  undo(state: ProjectState): ProjectState {
    return {
      ...state,
      clips: this.originalClips,
    };
  }

  serialize(): SerializedCommand {
    return {
      id: this.id,
      type: this.type,
      name: this.name,
      timestamp: this.timestamp,
      params: {
        clipId: this.clipId,
        ripple: this.ripple,
      },
    };
  }
}

/**
 * Concrete Command: Set Clip Gain
 */
export class SetClipGainCommand implements Command<ProjectState> {
  readonly id: string;
  readonly type = "clip.gain";
  readonly name: string;
  readonly timestamp: number;

  constructor(
    readonly clipId: string,
    readonly prevGainDb: number,
    readonly newGainDb: number,
    clipName?: string,
  ) {
    this.id = genId("gain");
    this.name = `Gain ${newGainDb >= 0 ? "+" : ""}${newGainDb.toFixed(1)} dB on "${clipName || clipId}"`;
    this.timestamp = Date.now();
  }

  do(state: ProjectState): ProjectState {
    const editor = new TimelineEditor(state);
    editor.setClipGain(this.clipId, this.newGainDb);
    return editor.getState();
  }

  undo(state: ProjectState): ProjectState {
    const editor = new TimelineEditor(state);
    editor.setClipGain(this.clipId, this.prevGainDb);
    return editor.getState();
  }

  merge(prev: Command<ProjectState>): Command<ProjectState> | null {
    if (prev instanceof SetClipGainCommand && prev.clipId === this.clipId) {
      if (this.timestamp - prev.timestamp < 1000) {
        return new SetClipGainCommand(this.clipId, prev.prevGainDb, this.newGainDb);
      }
    }
    return null;
  }

  serialize(): SerializedCommand {
    return {
      id: this.id,
      type: this.type,
      name: this.name,
      timestamp: this.timestamp,
      params: {
        clipId: this.clipId,
        prevGainDb: this.prevGainDb,
        newGainDb: this.newGainDb,
      },
    };
  }
}

/**
 * Concrete Command: Set Track Controls (Gain, Pan, Mute, Solo)
 */
export class SetTrackControlsCommand implements Command<ProjectState> {
  readonly id: string;
  readonly type = "track.controls";
  readonly name: string;
  readonly timestamp: number;

  constructor(
    readonly trackId: string,
    readonly prevSettings: Partial<Track>,
    readonly newSettings: Partial<Track>,
    trackName?: string,
  ) {
    this.id = genId("trk");
    const desc = Object.keys(newSettings).join(", ");
    this.name = `Adjust ${desc} on "${trackName || trackId}"`;
    this.timestamp = Date.now();
  }

  do(state: ProjectState): ProjectState {
    const editor = new TimelineEditor(state);
    editor.setTrackControls(this.trackId, this.newSettings);
    return editor.getState();
  }

  undo(state: ProjectState): ProjectState {
    const editor = new TimelineEditor(state);
    editor.setTrackControls(this.trackId, this.prevSettings);
    return editor.getState();
  }

  merge(prev: Command<ProjectState>): Command<ProjectState> | null {
    if (prev instanceof SetTrackControlsCommand && prev.trackId === this.trackId) {
      if (this.timestamp - prev.timestamp < 1000) {
        return new SetTrackControlsCommand(this.trackId, prev.prevSettings, {
          ...prev.newSettings,
          ...this.newSettings,
        });
      }
    }
    return null;
  }

  serialize(): SerializedCommand {
    return {
      id: this.id,
      type: this.type,
      name: this.name,
      timestamp: this.timestamp,
      params: {
        trackId: this.trackId,
        prevSettings: this.prevSettings,
        newSettings: this.newSettings,
      },
    };
  }
}

/**
 * Functional snapshot-based Generic Command for arbitrary operations
 */
export class FunctionalCommand implements Command<ProjectState> {
  readonly id: string;
  readonly type = "generic";
  readonly name: string;
  readonly timestamp: number;
  private prevState: ProjectState | null = null;

  constructor(
    name: string,
    private readonly executeFn: (s: ProjectState) => ProjectState,
  ) {
    this.id = genId("fn");
    this.name = name;
    this.timestamp = Date.now();
  }

  do(state: ProjectState): ProjectState {
    this.prevState = state;
    return this.executeFn(state);
  }

  undo(state: ProjectState): ProjectState {
    return this.prevState ?? state;
  }

  serialize(): SerializedCommand {
    return {
      id: this.id,
      type: this.type,
      name: this.name,
      timestamp: this.timestamp,
      params: {},
    };
  }
}

/**
 * CommandManager: Manages undo/redo stacks, branching visual history tree,
 * snapshots, coalescing, and state change listeners.
 */
export class CommandManager {
  private _state: ProjectState;
  private undoStack: Command<ProjectState>[] = [];
  private redoStack: Command<ProjectState>[] = [];

  private historyNodes: Map<string, HistoryNode> = new Map();
  private currentNodeId: string;
  private snapshots: ProjectSnapshot[] = [];
  private listeners: Set<(state: ProjectState) => void> = new Set();

  constructor(initialState: ProjectState) {
    this._state = initialState;
    const initialHash = computeProjectHash(initialState);
    const rootNode: HistoryNode = {
      id: "root",
      commandId: "init",
      name: "Initial Project",
      timestamp: Date.now(),
      parentId: null,
      childrenIds: [],
      stateHash: initialHash,
    };
    this.historyNodes.set("root", rootNode);
    this.currentNodeId = "root";
  }

  get state(): ProjectState {
    return this._state;
  }

  get currentHash(): string {
    return computeProjectHash(this._state);
  }

  get currentNode(): HistoryNode {
    return this.historyNodes.get(this.currentNodeId)!;
  }

  get canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  get canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  subscribe(listener: (state: ProjectState) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify(): void {
    for (const listener of this.listeners) {
      listener(this._state);
    }
  }

  execute(command: Command<ProjectState>): void {
    const lastCmd = this.undoStack[this.undoStack.length - 1];
    let mergedCmd: Command<ProjectState> | null = null;

    if (lastCmd && command.merge) {
      mergedCmd = command.merge(lastCmd);
    }

    if (mergedCmd) {
      this.undoStack[this.undoStack.length - 1] = mergedCmd;
      this._state = mergedCmd.do(this._state);

      const node = this.historyNodes.get(this.currentNodeId);
      if (node) {
        node.name = mergedCmd.name;
        node.stateHash = computeProjectHash(this._state);
      }
    } else {
      this._state = command.do(this._state);
      this.undoStack.push(command);
      this.redoStack = [];

      const newHash = computeProjectHash(this._state);
      const newNodeId = `node_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
      const isNewBranch =
        this.currentNode.childrenIds.length > 0 &&
        this.currentNode.id !== "root" &&
        this.undoStack.length === 1;

      const newNode: HistoryNode = {
        id: newNodeId,
        commandId: command.id,
        name: command.name,
        timestamp: Date.now(),
        parentId: this.currentNodeId,
        childrenIds: [],
        stateHash: newHash,
        branchName: isNewBranch ? `Branch: ${command.name}` : undefined,
      };

      this.historyNodes.set(newNodeId, newNode);
      this.currentNode.childrenIds.push(newNodeId);
      this.currentNodeId = newNodeId;
    }

    this.notify();
  }

  undo(): boolean {
    if (this.undoStack.length === 0) return false;

    const cmd = this.undoStack.pop()!;
    this._state = cmd.undo(this._state);
    this.redoStack.push(cmd);

    const curr = this.historyNodes.get(this.currentNodeId);
    if (curr && curr.parentId) {
      this.currentNodeId = curr.parentId;
    }

    this.notify();
    return true;
  }

  redo(): boolean {
    if (this.redoStack.length === 0) return false;

    const cmd = this.redoStack.pop()!;
    this._state = cmd.do(this._state);
    this.undoStack.push(cmd);

    const curr = this.historyNodes.get(this.currentNodeId);
    if (curr) {
      const child = curr.childrenIds
        .map((cid) => this.historyNodes.get(cid)!)
        .find((n) => n?.commandId === cmd.id);
      if (child) {
        this.currentNodeId = child.id;
      }
    }

    this.notify();
    return true;
  }

  createSnapshot(name: string): ProjectSnapshot {
    const snap: ProjectSnapshot = {
      id: `snap_${Date.now()}`,
      name,
      timestamp: Date.now(),
      stateHash: this.currentHash,
      state: JSON.parse(JSON.stringify(this._state)),
    };
    this.snapshots.push(snap);

    const curr = this.historyNodes.get(this.currentNodeId);
    if (curr) {
      curr.isSnapshot = true;
      curr.snapshotName = name;
    }

    return snap;
  }

  getSnapshots(): ProjectSnapshot[] {
    return [...this.snapshots];
  }

  getAllHistoryNodes(): HistoryNode[] {
    return Array.from(this.historyNodes.values());
  }

  getRecentEditsList(): { name: string; timeAgo: string; id: string; isCurrent: boolean }[] {
    const list: { name: string; timeAgo: string; id: string; isCurrent: boolean }[] = [];
    let curr: HistoryNode | undefined = this.historyNodes.get(this.currentNodeId);

    const now = Date.now();
    while (curr && curr.id !== "root") {
      const diffSec = Math.max(1, Math.round((now - curr.timestamp) / 1000));
      let timeAgo = `${diffSec} s ago`;
      if (diffSec >= 60) {
        timeAgo = `${Math.round(diffSec / 60)} min ago`;
      }
      if (diffSec >= 3600 * 24) {
        timeAgo = "Yesterday";
      }

      list.push({
        id: curr.id,
        name: curr.name,
        timeAgo,
        isCurrent: curr.id === this.currentNodeId,
      });

      curr = curr.parentId ? this.historyNodes.get(curr.parentId) : undefined;
    }

    return list;
  }
}
