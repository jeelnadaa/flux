import test from "node:test";
import assert from "node:assert/strict";
import { FluxPackage, type FluxPackageManifest } from "../FluxPackage.js";

test("FluxPackage: migrates older v1 schema to v2 with proper defaults", () => {
  const v1Raw = {
    name: "Legacy Project",
    tracks: [
      {
        id: "t1",
        name: "Vocals",
        gainDb: -2.0,
      },
    ],
    clips: [
      {
        id: "c1",
        trackId: "t1",
        name: "Vocal Take 1",
        startSeconds: 2.0,
        durationSeconds: 15.0,
      },
    ],
  };

  const migrated = FluxPackage.migrate(v1Raw);
  assert.equal(migrated.schemaVersion, 2);
  assert.equal(migrated.name, "Legacy Project");
  assert.equal(migrated.tracks.length, 1);
  assert.ok(migrated.tracks[0].color !== undefined);
  assert.deepEqual(migrated.tracks[0].automationLanes, []);

  assert.equal(migrated.clips[0].fadeInCurve, "equalPower");
  assert.equal(migrated.clips[0].fadeOutCurve, "equalPower");
  assert.ok(migrated.buses.length > 0);
  assert.ok(migrated.master !== undefined);
});

test("FluxPackage: serializes and deserializes cleanly in round trip", () => {
  const manifest: FluxPackageManifest = {
    format: "flux-package",
    schemaVersion: 2,
    id: "proj_roundtrip_1",
    name: "Harbor Lights",
    sampleRate: 48000,
    createdAt: 1700000000000,
    updatedAt: 1700000050000,
    tracks: [
      {
        id: "trk_1",
        name: "Vocals",
        color: "--t4",
        gainDb: 0,
        pan: -0.1,
        muted: false,
        soloed: false,
        armed: false,
        automationLanes: [],
      },
    ],
    clips: [
      {
        id: "c_1",
        trackId: "trk_1",
        name: "Chorus",
        sourceId: "src_vocal",
        sourceDurationSeconds: 60,
        sourceOffsetSeconds: 0,
        startSeconds: 42.318,
        durationSeconds: 20.0,
        gainDb: 0,
        reversed: false,
        fadeInDurationSeconds: 0.12,
        fadeInCurve: "equalPower",
        fadeOutDurationSeconds: 0.12,
        fadeOutCurve: "equalPower",
      },
    ],
    buses: [],
    master: { gainDb: -1.0, pan: 0, muted: false },
    mediaMap: {
      src_vocal: {
        id: "src_vocal",
        filename: "harbor-lights.flac",
        sizeBytes: 15000000,
        originalPath: "Sessions/June",
        status: "linked",
      },
    },
    snapshots: [],
  };

  const json = FluxPackage.serialize(manifest);
  const restored = FluxPackage.deserialize(json);

  assert.equal(restored.id, manifest.id);
  assert.equal(restored.name, manifest.name);
  assert.equal(restored.clips[0].startSeconds, 42.318);
  assert.equal(restored.mediaMap.src_vocal.filename, "harbor-lights.flac");
});

test("FluxPackage: detects missing media assets and identifies 'found nearby' vs 'not found'", () => {
  const manifest: FluxPackageManifest = {
    format: "flux-package",
    schemaVersion: 2,
    id: "proj_media",
    name: "Media Test",
    sampleRate: 48000,
    createdAt: 1000,
    updatedAt: 2000,
    tracks: [],
    clips: [],
    buses: [],
    master: { gainDb: 0, pan: 0, muted: false },
    mediaMap: {
      m1: {
        id: "m1",
        filename: "drums-final.wav",
        sizeBytes: 5000000,
        originalPath: "Sessions/June",
        status: "linked",
      },
      m2: {
        id: "m2",
        filename: "bass-di.wav",
        sizeBytes: 3000000,
        originalPath: "Sessions/June",
        status: "linked",
      },
    },
    snapshots: [],
  };

  const available = [{ name: "bass-di-take2.wav", path: "Audio/Stems/bass-di-take2.wav" }];

  const missing = FluxPackage.checkMissingMedia(manifest, available);
  assert.equal(missing.length, 2);

  const drums = missing.find((m) => m.filename === "drums-final.wav");
  assert.equal(drums?.status, "not_found");
  assert.equal(drums?.pillVariant, "warn");

  const bass = missing.find((m) => m.filename === "bass-di.wav");
  assert.equal(bass?.status, "found_nearby");
  assert.equal(bass?.pillVariant, "ok");
});

test("FluxPackage: relinks missing media files with updated file paths", () => {
  const manifest: FluxPackageManifest = {
    format: "flux-package",
    schemaVersion: 2,
    id: "proj_relink",
    name: "Relink Test",
    sampleRate: 48000,
    createdAt: 1000,
    updatedAt: 2000,
    tracks: [],
    clips: [],
    buses: [],
    master: { gainDb: 0, pan: 0, muted: false },
    mediaMap: {
      m1: {
        id: "m1",
        filename: "drums-final.wav",
        sizeBytes: 5000000,
        originalPath: "Sessions/June/drums-final.wav",
        status: "missing",
      },
    },
    snapshots: [],
  };

  const relinked = FluxPackage.relinkMedia(manifest, {
    m1: "/Volumes/Media/FluxProjects/drums-final.wav",
  });

  assert.equal(relinked.mediaMap.m1.status, "linked");
  assert.equal(relinked.mediaMap.m1.filename, "drums-final.wav");
  assert.equal(relinked.mediaMap.m1.originalPath, "/Volumes/Media/FluxProjects/drums-final.wav");
});
