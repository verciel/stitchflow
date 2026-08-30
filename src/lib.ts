import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";
import type {
  AiConfig,
  AiSuggestion,
  ArtworkAsset,
  BackupManifest,
  Collection,
  Design,
  DesignDetails,
  FilterOptions,
  GeneratedArtworkResult,
  ImportResult,
  InkstitchConfig,
  Job,
  ProposedEditResult,
  Tag,
} from "./types";


export const hasTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const MOCK_DESIGNS: Design[] = [
  {
    id: "sample-rose",
    title: "English Garden Rose",
    filename: "garden_rose.pes",
    format: "PES",
    widthMm: 82.0,
    heightMm: 76.0,
    stitches: 12480,
    colors: 5,
    sizeBytes: 18400,
    tags: ["floral", "rose", "botanical", "satin-stitch"],
    importedAt: "2026-08-28T12:00:00Z",
    duplicate: false,
    status: "active",
    aiCategory: "Floral & Botanical",
    aiSubject: "Garden Rose in Bloom",
    aiStyle: "Traditional Satin Stitch",
    aiDescription: "A multi-layered English garden rose featuring dense satin stitch petals and contoured botanical foliage.",
    dominantColors: ["#D32F2F", "#388E3C", "#B71C1C", "#1B5E20", "#FBC02D"],
    threads: [
      { index: 1, hex: "#D32F2F", brand: "Madeira Polyneon", description: "Classic Red" },
      { index: 2, hex: "#388E3C", brand: "Madeira Polyneon", description: "Leaf Green" },
      { index: 3, hex: "#B71C1C", brand: "Madeira Polyneon", description: "Deep Rose" },
      { index: 4, hex: "#1B5E20", brand: "Madeira Polyneon", description: "Forest Green" },
      { index: 5, hex: "#FBC02D", brand: "Madeira Polyneon", description: "Pollen Gold" },
    ],
  },
  {
    id: "sample-butterfly",
    title: "Meadow Monarch Butterfly",
    filename: "meadow_butterfly.dst",
    format: "DST",
    widthMm: 95.0,
    heightMm: 65.0,
    stitches: 8920,
    colors: 4,
    sizeBytes: 14200,
    tags: ["wildlife", "butterfly", "insects", "summer"],
    importedAt: "2026-08-28T12:00:00Z",
    duplicate: false,
    status: "active",
    aiCategory: "Wildlife & Nature",
    aiSubject: "Monarch Butterfly",
    aiStyle: "Tatami Fill & Satin Edge",
    aiDescription: "Intricate wing pattern with gradient tatami fill stitching and fine satin border contours.",
    dominantColors: ["#0288D1", "#7B1FA2", "#E91E63", "#FFD600"],
    threads: [
      { index: 1, hex: "#0288D1", brand: "Madeira Classic", description: "Sky Blue" },
      { index: 2, hex: "#7B1FA2", brand: "Madeira Classic", description: "Royal Purple" },
      { index: 3, hex: "#E91E63", brand: "Madeira Classic", description: "Magenta" },
      { index: 4, hex: "#FFD600", brand: "Madeira Classic", description: "Canary Yellow" },
    ],
  },
  {
    id: "sample-star",
    title: "Golden Celestial Starburst",
    filename: "little_star.jef",
    format: "JEF",
    widthMm: 50.0,
    heightMm: 50.0,
    stitches: 4310,
    colors: 2,
    sizeBytes: 8100,
    tags: ["star", "celestial", "gold", "geometric"],
    importedAt: "2026-08-28T12:00:00Z",
    duplicate: false,
    status: "active",
    aiCategory: "Geometric & Celestial",
    aiSubject: "Starburst Emblem",
    aiStyle: "Radiating Fill",
    aiDescription: "A symmetrical 8-point celestial star with radiating underlay and lustrous metallic gold topstitching.",
    dominantColors: ["#FBC02D", "#FF6F00"],
    threads: [
      { index: 1, hex: "#FBC02D", brand: "Robison-Anton", description: "Sun Gold" },
      { index: 2, hex: "#FF6F00", brand: "Robison-Anton", description: "Amber Gold" },
    ],
  },
  {
    id: "sample-crest",
    title: "Pacific Coast Crest",
    filename: "pacific_crest.pes",
    format: "PES",
    widthMm: 110.0,
    heightMm: 105.0,
    stitches: 18400,
    colors: 4,
    sizeBytes: 24100,
    tags: ["crest", "nautical", "varsity", "emblem"],
    importedAt: "2026-08-28T12:00:00Z",
    duplicate: false,
    status: "active",
    aiCategory: "Emblems & Badges",
    aiSubject: "Nautical Academy Crest",
    aiStyle: "Heavy Uniform Applique & Fill",
    aiDescription: "Traditional heraldic crest with anchor and laurel leaf wreath, digitized for heavy twill jacket backs.",
    dominantColors: ["#1A237E", "#00ACC1", "#FFB300", "#FFFFFF"],
    threads: [
      { index: 1, hex: "#1A237E", brand: "Madeira Polyneon", description: "Navy Blue" },
      { index: 2, hex: "#00ACC1", brand: "Madeira Polyneon", description: "Cyan Teal" },
      { index: 3, hex: "#FFB300", brand: "Madeira Polyneon", description: "Varsity Gold" },
      { index: 4, hex: "#FFFFFF", brand: "Madeira Polyneon", description: "Pure White" },
    ],
  },
  {
    id: "sample-mountain",
    title: "Nordic Mountain Sunset",
    filename: "nordic_peaks.exp",
    format: "EXP",
    widthMm: 100.0,
    heightMm: 85.0,
    stitches: 14200,
    colors: 4,
    sizeBytes: 19800,
    tags: ["mountains", "outdoor", "landscape", "adventure"],
    importedAt: "2026-08-28T12:00:00Z",
    duplicate: false,
    status: "active",
    aiCategory: "Landscapes & Outdoors",
    aiSubject: "Alpine Mountain Range",
    aiStyle: "Cross-hatch & Stepped Fill",
    aiDescription: "Geometric mountain peaks against a warm sunset horizon, optimized for knit beanies and outdoor gear.",
    dominantColors: ["#FF7043", "#3949AB", "#FFFFFF", "#2E7D32"],
    threads: [
      { index: 1, hex: "#FF7043", brand: "Madeira Classic", description: "Sunset Coral" },
      { index: 2, hex: "#3949AB", brand: "Madeira Classic", description: "Indigo Ridge" },
      { index: 3, hex: "#FFFFFF", brand: "Madeira Classic", description: "Alpine Snow" },
      { index: 4, hex: "#2E7D32", brand: "Madeira Classic", description: "Pine Needle" },
    ],
  },
];

const MOCK_COLLECTIONS: Collection[] = [
  {
    id: "col-botanical",
    name: "Botanical & Floral",
    description: "Nature and botanical floral embroidery motifs for spring collections.",
    createdAt: "2026-08-28T12:00:00Z",
    designCount: 2,
  },
  {
    id: "col-emblems",
    name: "Badges & Monograms",
    description: "Varsity jackets, nautical crests, and academy uniform insignia.",
    createdAt: "2026-08-28T12:00:00Z",
    designCount: 1,
  },
  {
    id: "col-outdoors",
    name: "Outdoor & Adventure",
    description: "Mountain ranges, celestial stars, and wildlife for outerwear.",
    createdAt: "2026-08-28T12:00:00Z",
    designCount: 2,
  },
];

const MOCK_TAGS: Tag[] = [
  { id: "tag-floral", name: "floral", count: 2 },
  { id: "tag-rose", name: "rose", count: 1 },
  { id: "tag-botanical", name: "botanical", count: 2 },
  { id: "tag-wildlife", name: "wildlife", count: 1 },
  { id: "tag-butterfly", name: "butterfly", count: 1 },
  { id: "tag-star", name: "star", count: 1 },
  { id: "tag-crest", name: "crest", count: 1 },
  { id: "tag-mountains", name: "mountains", count: 1 },
];

// Designs
export async function listDesigns(filters?: FilterOptions): Promise<Design[]> {
  if (!hasTauri()) {
    let list = [...MOCK_DESIGNS];
    if (filters?.format && filters.format !== "all") {
      list = list.filter((d) => d.format === filters.format);
    }
    if (filters?.tag) {
      list = list.filter((d) => d.tags.includes(filters.tag!));
    }
    if (filters?.query) {
      const q = filters.query.toLowerCase();
      list = list.filter(
        (d) =>
          d.title.toLowerCase().includes(q) ||
          d.filename.toLowerCase().includes(q) ||
          d.tags.some((t) => t.toLowerCase().includes(q))
      );
    }
    return list;
  }
  return invoke("list_designs", { filters });
}

export async function getDesignDetails(id: string): Promise<DesignDetails> {
  if (!hasTauri()) {
    const design = MOCK_DESIGNS.find((d) => d.id === id) || MOCK_DESIGNS[0];
    return {
      design,
      revisions: [
        {
          id: `rev-1-${design.id}`,
          designId: design.id,
          revisionNumber: 1,
          filename: design.filename,
          managedPath: `/managed/designs/${design.filename}`,
          checksum: `sha256-e9a3b8c4d7f1025a7b6c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9`,
          format: design.format,
          sizeBytes: design.sizeBytes,
          createdAt: design.importedAt,
          note: "Initial import",
        },
      ],
      linkedArtwork: [],
      linkedJobs: [],
      pendingSuggestions: [],
    };
  }
  return invoke("get_design_details", { id });
}



export async function findSimilarDesigns(designId: string, limit = 8): Promise<Design[]> {
  if (!hasTauri()) {
    return MOCK_DESIGNS.filter((d) => d.id !== designId).slice(0, limit);
  }
  return invoke("find_similar_designs", { designId, limit });
}

export async function updateDesignMetadata(

  id: string,
  title?: string,
  description?: string
): Promise<void> {
  return invoke("update_design_metadata", { id, title, description });
}

export async function confirmDialog(message: string, title = "Confirm Action"): Promise<boolean> {
  if (hasTauri()) {
    try {
      return await ask(message, { title, kind: "warning" });
    } catch {
      // Fallback to browser confirm if plugin dialog fails
    }
  }
  return typeof window !== "undefined" ? window.confirm(message) : true;
}

export async function deleteDesign(id: string): Promise<void> {
  if (!hasTauri()) return;
  return invoke("delete_design", { id });
}

export async function restoreDesign(id: string): Promise<void> {
  if (!hasTauri()) return;
  return invoke("restore_design", { id });
}

export async function permanentDeleteDesign(id: string): Promise<void> {
  if (!hasTauri()) return;
  return invoke("permanent_delete_design", { id });
}

export async function emptyRecycleBin(): Promise<number> {
  if (!hasTauri()) {
    return 0;
  }
  return invoke("empty_recycle_bin");
}

export async function exportDesign(
  id: string,
  targetPath: string,
  targetFormat: string
): Promise<void> {
  return invoke("export_design", { id, targetPath, targetFormat });
}

export async function revealInFolder(path: string): Promise<void> {
  return invoke("reveal_in_folder", { path });
}

// Imports
export async function importFiles(
  paths: string[],
  duplicatePolicy: "skip" | "keep_both" | "replace_revision"
): Promise<ImportResult[]> {
  return invoke("import_files", { paths, duplicatePolicy });
}

// Tags
export async function listTags(): Promise<Tag[]> {
  if (!hasTauri()) return MOCK_TAGS;
  return invoke("list_tags");
}


export async function addTagToDesign(designId: string, tagName: string): Promise<void> {
  return invoke("add_tag_to_design", { designId, tagName });
}

export async function removeTagFromDesign(designId: string, tagName: string): Promise<void> {
  return invoke("remove_tag_from_design", { designId, tagName });
}

// Collections
export async function listCollections(): Promise<Collection[]> {
  if (!hasTauri()) return MOCK_COLLECTIONS;
  return invoke("list_collections");
}

export async function createCollection(
  name: string,
  description?: string
): Promise<Collection> {
  return invoke("create_collection", { name, description });
}

export async function updateCollection(
  id: string,
  name: string,
  description?: string
): Promise<void> {
  return invoke("update_collection", { id, name, description });
}

export async function deleteCollection(id: string): Promise<void> {
  return invoke("delete_collection", { id });
}

export async function addDesignToCollection(
  collectionId: string,
  designId: string
): Promise<void> {
  return invoke("add_design_to_collection", { collectionId, designId });
}

export async function removeDesignFromCollection(
  collectionId: string,
  designId: string
): Promise<void> {
  return invoke("remove_design_from_collection", { collectionId, designId });
}

// Jobs
export async function listJobs(): Promise<Job[]> {
  if (!hasTauri()) {
    return [
      {
        id: "job-1",
        title: "Summer Polo Crest Run #408",
        notes: "Use 2.5oz cut-away backing and Madeira Polyneon thread.",
        status: "active",
        createdAt: "2026-08-28T12:00:00Z",
        updatedAt: "2026-08-28T12:00:00Z",
        designCount: 1,
        artworkCount: 1,
      },
    ];
  }
  return invoke("list_jobs");
}



export async function createJob(
  title: string,
  notes?: string,
  status?: string
): Promise<Job> {
  return invoke("create_job", { title, notes, status });
}

export async function updateJob(
  id: string,
  title: string,
  notes?: string,
  status?: string
): Promise<void> {
  return invoke("update_job", { id, title, notes, status });
}

export async function deleteJob(id: string): Promise<void> {
  return invoke("delete_job", { id });
}

export async function addDesignToJob(jobId: string, designId: string): Promise<void> {
  return invoke("add_design_to_job", { jobId, designId });
}

export async function removeDesignFromJob(jobId: string, designId: string): Promise<void> {
  return invoke("remove_design_from_job", { jobId, designId });
}

export async function addArtworkToJob(jobId: string, assetId: string): Promise<void> {
  return invoke("add_artwork_to_job", { jobId, assetId });
}

export async function removeArtworkFromJob(jobId: string, assetId: string): Promise<void> {
  return invoke("remove_artwork_from_job", { jobId, assetId });
}

// Artwork Assets
export async function listArtwork(): Promise<ArtworkAsset[]> {
  if (!hasTauri()) return [];
  return invoke("list_artwork");
}

export async function linkArtworkToDesign(designId: string, assetId: string): Promise<void> {
  return invoke("link_artwork_to_design", { designId, assetId });
}

export async function unlinkArtworkFromDesign(designId: string, assetId: string): Promise<void> {
  return invoke("unlink_artwork_from_design", { designId, assetId });
}

export async function deleteArtwork(assetId: string): Promise<void> {
  return invoke("delete_artwork", { assetId });
}

// Backups
export async function createBackup(destinationDir?: string): Promise<string> {
  return invoke("create_backup", { destinationDir });
}

export async function validateBackup(archivePath: string): Promise<BackupManifest> {
  return invoke("validate_backup", { archivePath });
}

export async function restoreBackup(
  archivePath: string,
  targetDirectory: string
): Promise<string> {
  return invoke("restore_backup", { archivePath, targetDirectory });
}

// Settings & Ink/Stitch
export async function getSettings(): Promise<Record<string, string>> {
  if (!hasTauri()) return {};
  return invoke("get_settings");
}

export async function saveSetting(key: string, value: string): Promise<void> {
  return invoke("save_setting", { key, value });
}

export async function getInkstitchConfig(): Promise<InkstitchConfig> {
  if (!hasTauri()) return { inkscapePath: "", isConfigured: false };
  return invoke("get_inkstitch_config");
}

export async function setInkstitchConfig(path: string): Promise<void> {
  return invoke("set_inkstitch_config", { path });
}

export async function openInInkstitch(designId: string): Promise<void> {
  return invoke("open_in_inkstitch", { designId });
}

// AI Integration
export async function getAiConfig(): Promise<AiConfig> {
  if (!hasTauri()) {
    return {
      endpoint: "https://api.openai.com/v1",
      model: "gpt-4o-mini",
      apiKey: "",
      enabled: false,
    };
  }
  return invoke("get_ai_config");
}

export async function saveAiConfig(config: AiConfig): Promise<void> {
  return invoke("save_ai_config", { config });
}

export async function testAiConnection(config: AiConfig): Promise<string> {
  return invoke("test_ai_connection", { config });
}

export async function testHfConnection(token?: string, model?: string): Promise<string> {
  if (!hasTauri()) {
    return "Connected to Hugging Face successfully!";
  }
  return invoke("test_hf_connection", { token, model });
}


export async function analyzeDesigns(designIds: string[]): Promise<AiSuggestion[]> {
  return invoke("analyze_designs", { designIds });
}

export async function applyAiSuggestion(id: string, accepted: boolean): Promise<void> {
  return invoke("apply_ai_suggestion", { id, accepted });
}

export async function naturalLanguageSearch(query: string): Promise<FilterOptions> {
  return invoke("natural_language_search", { query });
}

export async function getWorkflowAdvice(designId: string): Promise<string> {
  return invoke("get_workflow_advice", { designId });
}

export async function generateAiDesignImage(
  designId: string,
  customPrompt?: string,
  styleMode?: string
): Promise<GeneratedArtworkResult> {
  if (!hasTauri()) {
    return {
      imageData: "",
      tempPath: "/mock/ai_gen.png",
      promptUsed: "mock prompt",
    };
  }
  return invoke("generate_ai_design_image", { designId, customPrompt, styleMode });
}


export async function digitizeAndImportDesign(params: {
  sourceImagePath: string;
  title: string;
  targetFormat: string;
  widthMm: number;
  heightMm: number;
  tags: string[];
  category?: string;
}): Promise<Design> {
  if (!hasTauri()) {
    return MOCK_DESIGNS[0];
  }
  return invoke("digitize_and_import_design", {
    sourceImagePath: params.sourceImagePath,
    title: params.title,
    targetFormat: params.targetFormat,
    widthMm: params.widthMm,
    heightMm: params.heightMm,
    tags: params.tags,
    category: params.category,
  });
}

export async function proposeAiEdit(

  designId: string,
  instruction: string
): Promise<ProposedEditResult> {
  if (!hasTauri()) {
    throw new Error("Smart Edit requires desktop Tauri engine");
  }
  return invoke("propose_ai_edit", { designId, instruction });
}

export async function applyProposedEdit(params: {
  designId: string;
  tempEditedPath: string;
  tempPreviewPath: string;
  saveMode: "new_revision" | "new_design";
}): Promise<Design> {
  if (!hasTauri()) {
    return MOCK_DESIGNS[0];
  }
  return invoke("apply_proposed_edit", {
    designId: params.designId,
    tempEditedPath: params.tempEditedPath,
    tempPreviewPath: params.tempPreviewPath,
    saveMode: params.saveMode,
  });
}

// Utility



export async function readImageData(path: string): Promise<string> {
  return invoke("read_image_data", { path });
}

export function formatError(err: unknown, fallback = "An unexpected error occurred"): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object") {
    if ("message" in err && typeof (err as any).message === "string") return (err as any).message;
    if ("error" in err && typeof (err as any).error === "string") return (err as any).error;
  }
  return fallback;
}

