import { describe, expect, it } from "bun:test";
import {
  DOWNLOAD_SHARE,
  getBatchUpdateOverallProgress,
} from "@/lib/mods/update-progress";

const singleModProgress = (downloadPercentage: number) =>
  getBatchUpdateOverallProgress({
    completedMods: 0,
    totalMods: 1,
    downloadPercentage,
  });

describe("getBatchUpdateOverallProgress", () => {
  it("fills a single mod's download share as it downloads", () => {
    expect(
      getBatchUpdateOverallProgress({
        completedMods: 0,
        totalMods: 1,
        downloadPercentage: 50,
      }),
    ).toBeCloseTo(40);
  });

  it("offsets by the mods already completed", () => {
    expect(
      getBatchUpdateOverallProgress({
        completedMods: 1,
        totalMods: 2,
        downloadPercentage: 100,
      }),
    ).toBeCloseTo(90);
  });

  it("ends a finished download where the installing step starts", () => {
    const completedMods = 2;
    const totalMods = 4;
    const installingStart =
      (completedMods / totalMods) * 100 + (1 / totalMods) * 80;

    expect(DOWNLOAD_SHARE).toBe(0.8);
    expect(
      getBatchUpdateOverallProgress({
        completedMods,
        totalMods,
        downloadPercentage: 100,
      }),
    ).toBeCloseTo(installingStart);
  });

  it("clamps out-of-range and non-finite percentages", () => {
    expect(singleModProgress(150)).toBeCloseTo(80);
    expect(singleModProgress(-10)).toBe(0);
    expect(singleModProgress(Number.NaN)).toBe(0);
  });

  it("returns 0 when there are no mods", () => {
    expect(
      getBatchUpdateOverallProgress({
        completedMods: 0,
        totalMods: 0,
        downloadPercentage: 50,
      }),
    ).toBe(0);
  });
});
