export type ThreadInfo = {
  index: number;
  hex: string;
  brand: string;
  description: string;
};

export type Design = {
  id: string;
  title: string;
  filename: string;
  format: string;
  widthMm?: number;
  heightMm?: number;
  stitches?: number;
  colors?: number;
  sizeBytes: number;
  tags: string[];
  collection?: string;
  collectionId?: string;
  job?: string;
  jobId?: string;
  importedAt: string;
  duplicate: boolean;
  previewUrl?: string;
  previewPath?: string;
  managedPath?: string;
  status: "active" | "recycled";
  aiCategory?: string;
  aiSubject?: string;
  aiStyle?: string;
  aiDescription?: string;
  dominantColors: string[];
  threads: ThreadInfo[];
};

export type DesignRevision = {
  id: string;
  designId: string;
  revisionNumber: number;
  filename: string;
  managedPath: string;
  checksum: string;
  format: string;
  sizeBytes: number;
  createdAt: string;
  note: string;
};

export type ArtworkAsset = {
  id: string;
  filename: string;
  managedPath: string;
  previewUrl?: string;
  checksum: string;
  mimeType: string;
  sizeBytes: number;
  sourcePath?: string;
  importedAt: string;
  status: "active" | "recycled";
};

export type Collection = {
  id: string;
  name: string;
  description: string;
  designCount: number;
  createdAt: string;
};

export type Job = {
  id: string;
  title: string;
  notes: string;
  status: "draft" | "active" | "completed" | "archived";
  designCount: number;
  artworkCount: number;
  createdAt: string;
  updatedAt: string;
};

export type Tag = {
  id: string;
  name: string;
  count: number;
};

export type ImportResult = {
  path: string;
  status: "imported" | "duplicate" | "unsupported" | "invalid" | "failed";
  design?: Design;
  message?: string;
};

export type AiSuggestion = {
  id: string;
  designId: string;
  category?: string;
  subject?: string;
  style?: string;
  description?: string;
  tags: string[];
  dominantColors: string[];
  confidence: number;
  status: "pending" | "accepted" | "dismissed";
  provider?: string;
  model?: string;
  createdAt: string;
};

export type AiConfig = {
  endpoint: string;
  model: string;
  apiKey: string;
  enabled: boolean;
};

export type InkstitchConfig = {
  inkscapePath: string;
  isConfigured: boolean;
};

export type DesignDetails = {
  design: Design;
  revisions: DesignRevision[];
  linkedArtwork: ArtworkAsset[];
  linkedJobs: Job[];
  pendingSuggestions: AiSuggestion[];
};

export type FilterOptions = {
  query?: string;
  tag?: string;
  format?: string;
  collectionId?: string;
  jobId?: string;
  status?: string;
  sortBy?: "date_desc" | "date_asc" | "stitches_desc" | "stitches_asc" | "title_asc" | "size_desc";
};

export type BackupManifest = {
  version: string;
  createdAt: string;
  appVersion: string;
  designCount: number;
  artworkCount: number;
  fileChecksums: Record<string, string>;
};
