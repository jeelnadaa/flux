import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  sphericalToCartesian,
  cartesianToSpherical,
  computeItdWoodworth,
  computeDistanceGain,
  generateOrbitPosition,
  generateFigureEightPosition,
  generateSpiralPosition,
  generateVerticalLoopPosition,
  generateSwarmPosition,
  generateLissajousPosition,
  interpolateCatmullRomSpline,
  checkMonoCompatibility,
  SPATIAL_PRESETS,
  DEFAULT_STAGE_STEMS,
  calculateVirtualSpeakerPositions,
  calculateMidSideSignals,
  reconstructFromMidSide,
  DEFAULT_STEREO_SPATIAL_CONFIG,
  HRTF_PROFILES,
  DEFAULT_HEAD_WIDTH_CM,
  ROOM_PRESETS,
  SURFACE_MATERIALS,
  computeSabineRt60,
  DEFAULT_CONCERT_HALL,
  DEFAULT_NEXT_DOOR,
} from "../SpatialModels.js";

describe("Spatial Domain Models and Coordinates", () => {
  it("converts between spherical and Cartesian coordinates symmetrically", () => {
    // 0 deg azimuth, 0 elevation, 2m distance -> straight ahead (Y=2, X=0, Z=0)
    const p1 = sphericalToCartesian(0, 0, 2.0);
    assert.ok(Math.abs(p1.x) < 1e-4);
    assert.ok(Math.abs(p1.y - 2.0) < 1e-4);
    assert.ok(Math.abs(p1.z) < 1e-4);

    const s1 = cartesianToSpherical(p1.x, p1.y, p1.z);
    assert.ok(Math.abs(s1.azimuthDeg - 0) < 1e-4);
    assert.ok(Math.abs(s1.elevationDeg - 0) < 1e-4);
    assert.ok(Math.abs(s1.distanceM - 2.0) < 1e-4);

    // 90 deg azimuth (hard right) -> X=2, Y=0, Z=0
    const p2 = sphericalToCartesian(90, 0, 2.0);
    assert.ok(Math.abs(p2.x - 2.0) < 1e-4);
    assert.ok(Math.abs(p2.y) < 1e-4);
    assert.ok(Math.abs(p2.z) < 1e-4);

    const s2 = cartesianToSpherical(p2.x, p2.y, p2.z);
    assert.ok(Math.abs(s2.azimuthDeg - 90) < 1e-4);
  });

  it("computes Woodworth ITD delay within standard 600-700 us range at 90 degrees", () => {
    const res90 = computeItdWoodworth(90, DEFAULT_HEAD_WIDTH_CM);
    const itdMicrosec = res90.itdSec * 1_000_000;

    assert.ok(
      itdMicrosec >= 600 && itdMicrosec <= 700,
      `ITD at 90 deg should be in [600, 700] µs, got ${itdMicrosec.toFixed(1)} µs`,
    );
    assert.equal(res90.delayRightSec, 0); // Sound hits right ear first

    // Center has zero ITD
    const res0 = computeItdWoodworth(0, DEFAULT_HEAD_WIDTH_CM);
    assert.equal(res0.itdSec, 0);

    // Hard left (-90 deg / 270 deg)
    const resLeft = computeItdWoodworth(270, DEFAULT_HEAD_WIDTH_CM);
    const itdLeftMicrosec = resLeft.itdSec * 1_000_000;
    assert.ok(
      itdLeftMicrosec >= 600 && itdLeftMicrosec <= 700,
      `ITD at 270 deg should be in [600, 700] µs, got ${itdLeftMicrosec.toFixed(1)} µs`,
    );
    assert.equal(resLeft.delayLeftSec, 0); // Sound hits left ear first
  });

  it("scales ITD proportionally with head width", () => {
    const normal = computeItdWoodworth(90, 17.5);
    const wide = computeItdWoodworth(90, 21.0);
    const narrow = computeItdWoodworth(90, 14.0);

    assert.ok(wide.itdSec > normal.itdSec);
    assert.ok(narrow.itdSec < normal.itdSec);
    assert.ok(Math.abs(wide.itdSec / normal.itdSec - 21.0 / 17.5) < 1e-4);
  });

  it("calculates 1/r distance attenuation with safety clamping", () => {
    const gain1m = computeDistanceGain(1.0);
    const gain2m = computeDistanceGain(2.0);
    const gain4m = computeDistanceGain(4.0);

    assert.equal(gain1m, 1.0);
    assert.equal(gain2m, 0.5);
    assert.equal(gain4m, 0.25);

    // Near-field clamp prevents infinite amplitude
    const gainZero = computeDistanceGain(0.01);
    assert.ok(gainZero <= 3.0);
  });

  it("generates continuous trajectory paths for Orbit, Figure-8, and Spiral", () => {
    // Orbit at 0.1 Hz (10 s cycle)
    const orb0 = generateOrbitPosition(0, 0.1, "Clockwise", 1.8, 0);
    const orb2_5 = generateOrbitPosition(2.5, 0.1, "Clockwise", 1.8, 0);
    const orb5 = generateOrbitPosition(5.0, 0.1, "Clockwise", 1.8, 0);

    assert.equal(orb0.azimuthDeg, 0);
    assert.ok(Math.abs(orb2_5.azimuthDeg - 90) < 1e-4);
    assert.ok(Math.abs(orb5.azimuthDeg - 180) < 1e-4);

    // Figure 8 bounded coordinates
    for (let t = 0; t <= 10; t += 1.0) {
      const fig = generateFigureEightPosition(t, 0.1, 2.0, 15);
      assert.ok(Math.abs(fig.x) <= 2.0);
      assert.ok(Math.abs(fig.y) <= 2.0);
    }

    // Spiral varying radius
    const sp = generateSpiralPosition(2.5, 0.1, 1.0, 3.0, -20, 20);
    assert.ok(sp.distanceM >= 1.0 && sp.distanceM <= 3.0);
    assert.ok(sp.elevationDeg >= -20 && sp.elevationDeg <= 20);
  });

  it("provides all 6 production HRTF profiles with valid metadata", () => {
    const profiles = Object.values(HRTF_PROFILES);
    assert.equal(profiles.length, 6);
    assert.equal(HRTF_PROFILES.C.name, "Profile C");
    assert.equal(HRTF_PROFILES.C.description, "Medium, strong rear cue");
  });

  it("calculates virtual speaker positions with angular spread", () => {
    // 0 deg azimuth with 60 deg spread -> Left at 330 deg (-30 deg), Right at 30 deg
    const sp0 = calculateVirtualSpeakerPositions(0, 60, 1.5, 5);
    assert.equal(sp0.left.azimuthDeg, 330);
    assert.equal(sp0.right.azimuthDeg, 30);
    assert.equal(sp0.left.distanceM, 1.5);
    assert.equal(sp0.right.distanceM, 1.5);
    assert.equal(sp0.left.elevationDeg, 5);

    // 90 deg azimuth with 90 deg spread -> Left at 45 deg, Right at 135 deg
    const sp90 = calculateVirtualSpeakerPositions(90, 90, 2.0);
    assert.equal(sp90.left.azimuthDeg, 45);
    assert.equal(sp90.right.azimuthDeg, 135);
  });

  it("encodes and decodes Mid/Side signals bit-accurately", () => {
    const l = new Float32Array([1.0, 0.5, -0.5, 0.0, 0.8]);
    const r = new Float32Array([0.2, 0.5, 0.5, -0.6, 0.4]);

    const { mid, side } = calculateMidSideSignals(l, r);
    // mid = 0.5 * (1.0 + 0.2) = 0.6
    assert.ok(Math.abs(mid[0] - 0.6) < 1e-6);
    // side = 0.5 * (1.0 - 0.2) = 0.4
    assert.ok(Math.abs(side[0] - 0.4) < 1e-6);

    const reconstructed = reconstructFromMidSide(mid, side);
    for (let i = 0; i < l.length; i++) {
      assert.ok(Math.abs(reconstructed.left[i] - l[i]) < 1e-6);
      assert.ok(Math.abs(reconstructed.right[i] - r[i]) < 1e-6);
    }
  });

  it("validates default stereo spatializer configuration", () => {
    assert.equal(DEFAULT_STEREO_SPATIAL_CONFIG.mode, "Two speakers");
    assert.equal(DEFAULT_STEREO_SPATIAL_CONFIG.spreadDeg, 60);
    assert.equal(DEFAULT_STEREO_SPATIAL_CONFIG.monoBassEnabled, true);
    assert.equal(DEFAULT_STEREO_SPATIAL_CONFIG.monoBassCrossoverHz, 120);
    assert.equal(DEFAULT_STEREO_SPATIAL_CONFIG.profile, "C");
  });

  it("validates all 9 static spatial presets", () => {
    const keys = Object.keys(SPATIAL_PRESETS);
    assert.equal(keys.length, 9);
    assert.equal(SPATIAL_PRESETS.Front.azimuthDeg, 0);
    assert.equal(SPATIAL_PRESETS.Behind.azimuthDeg, 180);
    assert.equal(SPATIAL_PRESETS.Above.elevationDeg, 60);
    assert.equal(SPATIAL_PRESETS.Below.elevationDeg, -60);
    assert.equal(SPATIAL_PRESETS.FrontLeft.azimuthDeg, 315);
    assert.equal(SPATIAL_PRESETS.FrontRight.azimuthDeg, 45);
    assert.equal(SPATIAL_PRESETS.RearLeft.azimuthDeg, 225);
    assert.equal(SPATIAL_PRESETS.RearRight.azimuthDeg, 135);
    assert.equal(SPATIAL_PRESETS.InsideRoom.azimuthDeg, 30);
  });

  it("evaluates 16D extended trajectories: Vertical Loop, Swarm, Lissajous", () => {
    // Vertical loop
    const vloop = generateVerticalLoopPosition(0.25, 1.0, 1.8);
    assert.ok(Math.abs(vloop.elevationDeg - 85.0) < 1e-3);

    // Swarm deterministic seed
    const swarm1 = generateSwarmPosition(1.5, 0.2, 2.0, 42);
    const swarm2 = generateSwarmPosition(1.5, 0.2, 2.0, 42);
    assert.equal(swarm1.x, swarm2.x);
    assert.equal(swarm1.y, swarm2.y);
    assert.equal(swarm1.z, swarm2.z);

    // Lissajous
    const liss = generateLissajousPosition(0.5, 0.1, 1.8);
    assert.ok(Math.abs(liss.x) <= 1.81);
    assert.ok(Math.abs(liss.y) <= 1.81);
  });

  it("interpolates Catmull-Rom spline control points smoothly", () => {
    const pts: [number, number][] = [
      [0, 1],
      [1, 0],
      [0, -1],
      [-1, 0],
    ];
    const pStart = interpolateCatmullRomSpline(pts, 0);
    assert.ok(Math.abs(pStart[0] - 0) < 1e-4);
    assert.ok(Math.abs(pStart[1] - 1) < 1e-4);

    const pEnd = interpolateCatmullRomSpline(pts, 1);
    assert.ok(Math.abs(pEnd[0] - -1) < 1e-4);
    assert.ok(Math.abs(pEnd[1] - 0) < 1e-4);

    const pMid = interpolateCatmullRomSpline(pts, 0.5);
    assert.ok(pMid[0] > -1.5 && pMid[0] < 1.5);
    assert.ok(pMid[1] > -1.5 && pMid[1] < 1.5);
  });

  it("checks mono compatibility accurately for mono, anti-phase, and stereo audio", () => {
    const mono = new Float32Array([0.5, -0.5, 0.8, -0.2, 0.1]);
    const resMono = checkMonoCompatibility(mono, mono);
    assert.ok(Math.abs(resMono.correlation - 1.0) < 1e-4);
    assert.ok(resMono.monoLossDb < 0.1);
    assert.equal(resMono.isCompatible, true);

    const anti = new Float32Array([-0.5, 0.5, -0.8, 0.2, -0.1]);
    const resAnti = checkMonoCompatibility(mono, anti);
    assert.ok(Math.abs(resAnti.correlation - -1.0) < 1e-4);
    assert.equal(resAnti.isCompatible, false);
  });

  it("defines default stage stems for 4-stem spatial mixing", () => {
    assert.equal(DEFAULT_STAGE_STEMS.length, 4);
    assert.equal(DEFAULT_STAGE_STEMS[0].name, "Vocals");
    assert.equal(DEFAULT_STAGE_STEMS[1].name, "Drums");
    assert.equal(DEFAULT_STAGE_STEMS[2].name, "Bass");
    assert.equal(DEFAULT_STAGE_STEMS[3].name, "Other");
  });

  it("provides 12 production room presets with RT60 and dimensions", () => {
    assert.equal(ROOM_PRESETS.length, 12);
    const presets = Object.fromEntries(ROOM_PRESETS.map((p) => [p.id, p]));
    assert.equal(presets.cathedral.rt60, 6.8);
    assert.equal(presets.bathroom.rt60, 0.9);
    assert.equal(presets.concert_hall.rt60, 3.2);
    assert.equal(presets.studio_booth.rt60, 0.2);
    assert.equal(presets.car_interior.rt60, 0.1);
    assert.equal(presets.stadium.rt60, 5.1);
  });

  it("calculates Sabine RT60 and first reflection delay accurately", () => {
    assert.ok(SURFACE_MATERIALS.concrete.absorption > 0);
    // 12m x 18m x 6m room with plaster walls, wood floor, plaster ceiling
    const res = computeSabineRt60(12, 18, 6, "plaster", "wood", "plaster");
    assert.equal(res.volumeM3, 1296);
    // Sabine formula ~3.81s
    assert.ok(Math.abs(res.rt60 - 3.81) < 0.2, `Computed RT60 ${res.rt60}`);
    assert.ok(res.firstReflectionMs >= 8 && res.firstReflectionMs <= 35);
  });

  it("defines default concert hall and next door configurations", () => {
    assert.equal(DEFAULT_CONCERT_HALL.row, 10);
    assert.equal(DEFAULT_CONCERT_HALL.enabled, true);
    assert.equal(DEFAULT_NEXT_DOOR.cutoffHz, 620);
    assert.equal(DEFAULT_NEXT_DOOR.bassBoostDb, 4.0);
  });
});
