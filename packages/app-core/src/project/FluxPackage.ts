import type { Track, Clip, Bus } from "../editing/EditingModel.js";
import type { ProjectSnapshot } from "./Command.js";

export interface MediaAssetInfo {
  id: string;
  filename: string;
  sizeBytes: number;
  originalPath?: string;
  status: "bundled" | "linked" | "missing";
}

export interface FluxPackageManifest {
  format: "flux-package";
  schemaVersion: number;
  id: string;
  name: string;
  sampleRate: number;
  createdAt: number;
  updatedAt: number;
  tracks: Track[];
  clips: Clip[];
  buses: Bus[];
  master: {
    gainDb: number;
    pan: number;
    muted: boolean;
  };
  mediaMap: Record<string, MediaAssetInfo>;
  snapshots: ProjectSnapshot[];
}

export interface MissingMediaItem {
  id: string;
  filename: string;
  originalPath: string;
  status: "not_found" | "found_nearby";
  statusLabel: string;
  pillVariant: "warn" | "ok";
  matchedPath?: string;
}

export class FluxPackage {
  static readonly CURRENT_SCHEMA_VERSION = 2;

  /**
   * Migrate an older schema (e.g. v1) to current v2 format
   */
  static migrate(raw: Record<string, unknown>): FluxPackageManifest {
    const rawTracks = (raw.tracks as Record<string, unknown>[]) || [];
    let tracks: Track[] = rawTracks.map((t) => ({
      id: (t.id as string) || `trk_${Math.random().toString(36).slice(2, 6)}`,
      name: (t.name as string) || "Track",
      color: (t.color as string) || "--t1",
      gainDb: (t.gainDb as number) ?? 0.0,
      pan: (t.pan as number) ?? 0.0,
      muted: Boolean(t.muted),
      soloed: Boolean(t.soloed),
      armed: Boolean(t.armed),
      tall: Boolean(t.tall),
      busId: t.busId as string | undefined,
      automationLanes: (t.automationLanes as Track["automationLanes"]) || [],
    }));

    if (tracks.length === 0) {
      tracks = [
        {
          id: "trk_1",
          name: "Vocals",
          color: "--t4",
          gainDb: 0,
          pan: 0,
          muted: false,
          soloed: false,
          armed: false,
          automationLanes: [],
        },
      ];
    }

    const rawClips = (raw.clips as Record<string, unknown>[]) || [];
    const clips: Clip[] = rawClips.map((c) => ({
      id: (c.id as string) || `clip_${Math.random().toString(36).slice(2, 6)}`,
      trackId: (c.trackId as string) || tracks[0].id,
      name: (c.name as string) || "Audio Clip",
      sourceId: (c.sourceId as string) || "default_source",
      sourceDurationSeconds:
        (c.sourceDurationSeconds as number) || (c.durationSeconds as number) || 10,
      sourceOffsetSeconds: (c.sourceOffsetSeconds as number) || 0,
      startSeconds: (c.startSeconds as number) || 0,
      durationSeconds: (c.durationSeconds as number) || 10,
      gainDb: (c.gainDb as number) ?? 0.0,
      reversed: Boolean(c.reversed),
      fadeInDurationSeconds: (c.fadeInDurationSeconds as number) || 0.01,
      fadeInCurve: (c.fadeInCurve as Clip["fadeInCurve"]) || "equalPower",
      fadeOutDurationSeconds: (c.fadeOutDurationSeconds as number) || 0.01,
      fadeOutCurve: (c.fadeOutCurve as Clip["fadeOutCurve"]) || "equalPower",
      selected: Boolean(c.selected),
    }));

    const buses: Bus[] = (raw.buses as Bus[]) || [
      {
        id: "bus_music",
        name: "Music bus",
        gainDb: 0,
        pan: 0,
        muted: false,
        soloed: false,
      },
    ];

    const master = (raw.master as FluxPackageManifest["master"]) || {
      gainDb: 0,
      pan: 0,
      muted: false,
    };

    const mediaMap = (raw.mediaMap as Record<string, MediaAssetInfo>) || {};
    const snapshots = (raw.snapshots as ProjectSnapshot[]) || [];

    return {
      format: "flux-package",
      schemaVersion: FluxPackage.CURRENT_SCHEMA_VERSION,
      id: (raw.id as string) || `proj_${Date.now()}`,
      name: (raw.name as string) || "Untitled Project",
      sampleRate: (raw.sampleRate as number) || 48000,
      createdAt: (raw.createdAt as number) || Date.now(),
      updatedAt: Date.now(),
      tracks,
      clips,
      buses,
      master,
      mediaMap,
      snapshots,
    };
  }

  /**
   * Serialize project to a .flux package string (or Blob)
   */
  static serialize(manifest: FluxPackageManifest): string {
    return JSON.stringify(manifest, null, 2);
  }

  /**
   * Open and deserialize a .flux package, migrating if necessary
   */
  static deserialize(rawJson: string): FluxPackageManifest {
    const raw = JSON.parse(rawJson);
    return FluxPackage.migrate(raw);
  }

  /**
   * Check project media assets against available filenames/paths
   */
  static checkMissingMedia(
    manifest: FluxPackageManifest,
    availableFiles: { name: string; path?: string }[],
  ): MissingMediaItem[] {
    const availableNames = new Set(availableFiles.map((f) => f.name.toLowerCase()));
    const missing: MissingMediaItem[] = [];

    for (const [id, asset] of Object.entries(manifest.mediaMap)) {
      const lower = asset.filename.toLowerCase();
      if (!availableNames.has(lower)) {
        // Check if there's a nearby match (e.g. same name in different directory or similar base)
        const nearby = availableFiles.find(
          (f) =>
            f.name.toLowerCase().includes(lower.replace(/\.[^.]+$/, "")) ||
            lower.includes(f.name.toLowerCase().replace(/\.[^.]+$/, "")),
        );

        if (nearby) {
          missing.push({
            id,
            filename: asset.filename,
            originalPath: asset.originalPath || "Sessions/June",
            status: "found_nearby",
            statusLabel: "Found nearby",
            pillVariant: "ok",
            matchedPath: nearby.path || nearby.name,
          });
        } else {
          missing.push({
            id,
            filename: asset.filename,
            originalPath: asset.originalPath || "Sessions/June",
            status: "not_found",
            statusLabel: "Not found",
            pillVariant: "warn",
          });
        }
      }
    }

    return missing;
  }

  /**
   * Relink missing media files with updated paths
   */
  static relinkMedia(
    manifest: FluxPackageManifest,
    relinkMapping: Record<string, string>,
  ): FluxPackageManifest {
    const updatedMediaMap = { ...manifest.mediaMap };

    for (const [id, newPath] of Object.entries(relinkMapping)) {
      if (updatedMediaMap[id]) {
        const filename = newPath.split(/[/\\]/).pop() || newPath;
        updatedMediaMap[id] = {
          ...updatedMediaMap[id],
          filename,
          originalPath: newPath,
          status: "linked",
        };
      }
    }

    return {
      ...manifest,
      mediaMap: updatedMediaMap,
      updatedAt: Date.now(),
    };
  }
}
