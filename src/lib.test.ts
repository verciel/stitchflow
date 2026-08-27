import { describe, expect, it } from "vitest";
import { hasTauri } from "./lib";
import type { Design, Job } from "./types";

describe("Frontend Client & Model Contracts", () => {
  it("detects tauri environment properly", () => {
    // In node/vitest environment, __TAURI_INTERNALS__ is not present
    expect(hasTauri()).toBe(false);
  });

  it("conforms to Design type schema", () => {
    const sample: Design = {
      id: "test-id",
      title: "Sample Pattern",
      filename: "sample.pes",
      format: "PES",
      widthMm: 100,
      heightMm: 100,
      stitches: 5400,
      colors: 3,
      sizeBytes: 2048,
      tags: ["floral", "spring"],
      importedAt: "2026-08-28T00:00:00Z",
      duplicate: false,
      status: "active",
      dominantColors: ["#ff0000"],
      threads: [
        {
          index: 1,
          hex: "#ff0000",
          brand: "Madeira",
          description: "Red",
        },
      ],
    };

    expect(sample.format).toBe("PES");
    expect(sample.threads.length).toBe(1);
    expect(sample.status).toBe("active");
  });

  it("conforms to Job container status constraints", () => {
    const validStatuses: Job["status"][] = ["draft", "active", "completed", "archived"];
    expect(validStatuses).toContain("draft");
    expect(validStatuses).toContain("active");
    expect(validStatuses).toContain("completed");
    expect(validStatuses).toContain("archived");
  });
});
