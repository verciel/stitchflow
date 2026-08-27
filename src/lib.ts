import { invoke } from "@tauri-apps/api/core";
import type {
  AiConfig,
  AiSuggestion,
  ArtworkAsset,
  BackupManifest,
  Collection,
  Design,
  DesignDetails,
  FilterOptions,
  ImportResult,
  InkstitchConfig,
  Job,
  Tag,
} from "./types";

export const hasTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Designs
export async function listDesigns(filters?: FilterOptions): Promise<Design[]> {
  if (!hasTauri()) return [];
  return invoke("list_designs", { filters });
}

export async function getDesignDetails(id: string): Promise<DesignDetails> {
  return invoke("get_design_details", { id });
}

export async function updateDesignMetadata(
  id: string,
  title?: string,
  description?: string
): Promise<void> {
  return invoke("update_design_metadata", { id, title, description });
}

export async function deleteDesign(id: string): Promise<void> {
  return invoke("delete_design", { id });
}

export async function restoreDesign(id: string): Promise<void> {
  return invoke("restore_design", { id });
}

export async function permanentDeleteDesign(id: string): Promise<void> {
  return invoke("permanent_delete_design", { id });
}

export async function emptyRecycleBin(): Promise<number> {
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
  if (!hasTauri()) return [];
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
  if (!hasTauri()) return [];
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
  if (!hasTauri()) return [];
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

