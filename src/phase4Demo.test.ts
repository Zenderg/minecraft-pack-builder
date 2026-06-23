import { describe, expect, it } from "vitest";

import { browserDomainDemoReport, getDomainDemoStats } from "./phase4Demo";

describe("phase 4 domain demo presentation", () => {
  it("summarizes dimensions, stages, blocks, materials, and rejected actions", () => {
    expect(getDomainDemoStats(browserDomainDemoReport)).toEqual([
      ["Dimensions", "4 x 3 x 4"],
      ["Stages", "3"],
      ["Blocks", "6"],
      ["Materials", "4"],
      ["Rejected actions", "2"],
    ]);
  });

  it("keeps unassigned materials in final counts", () => {
    expect(browserDomainDemoReport.materials).toContainEqual({
      blockId: "create:andesite_casing",
      count: 1,
    });
  });
});
