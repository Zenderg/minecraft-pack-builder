export type DomainDemoReport = {
  schemeName: string;
  summary: {
    dimensions: { x: number; y: number; z: number };
    stageCount: number;
    blockCount: number;
    materialCount: number;
  };
  stages: Array<{ id: number | null; name: string; order: number | null }>;
  materials: Array<{ blockId: string; count: number }>;
  rejectedActions: Array<{ action: string; code: string; message: string }>;
};

export type DomainDemoArtifact = {
  path: string | null;
  report: DomainDemoReport;
};

export const browserDomainDemoReport: DomainDemoReport = {
  schemeName: "Domain Demo Scheme",
  summary: {
    dimensions: { x: 4, y: 3, z: 4 },
    stageCount: 3,
    blockCount: 6,
    materialCount: 4,
  },
  stages: [
    { id: 1, name: "Foundation", order: 1 },
    { id: 2, name: "Machines", order: 2 },
    { id: null, name: "Unassigned", order: null },
  ],
  materials: [
    { blockId: "create:andesite_casing", count: 1 },
    { blockId: "minecraft:glass", count: 2 },
    { blockId: "minecraft:stone_bricks", count: 2 },
    { blockId: "thermal:machine_frame", count: 1 },
  ],
  rejectedActions: [
    {
      action: "place missing block",
      code: "unknown_block",
      message: "unknown block id minecraft:missing_block",
    },
    {
      action: "bulk set out of bounds",
      code: "coordinate_out_of_bounds",
      message: "coordinate (4, 0, 0) is outside 4 x 3 x 4",
    },
  ],
};

export const browserDomainDemoArtifact: DomainDemoArtifact = {
  path: null,
  report: browserDomainDemoReport,
};

export function getDomainDemoStats(report: DomainDemoReport): Array<[string, string]> {
  const { dimensions } = report.summary;
  return [
    ["Dimensions", `${dimensions.x} x ${dimensions.y} x ${dimensions.z}`],
    ["Stages", String(report.summary.stageCount)],
    ["Blocks", String(report.summary.blockCount)],
    ["Materials", String(report.summary.materialCount)],
    ["Rejected actions", String(report.rejectedActions.length)],
  ];
}
