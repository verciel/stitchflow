import React, { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  Check,
  Copy,
  ExternalLink,
  FileDown,
  FolderOpen,
  History,
  Image as ImageIcon,
  Plus,
  RefreshCw,
  Sparkles,
  Tag,
  Trash2,
  Undo2,
  X,
} from "lucide-react";
import {
  addDesignToCollection,
  addDesignToJob,
  addTagToDesign,
  exportDesign,
  formatError,
  getDesignDetails,
  linkArtworkToDesign,
  listArtwork,
  listCollections,
  listJobs,
  openInInkstitch,
  permanentDeleteDesign,
  removeDesignFromCollection,
  removeDesignFromJob,
  removeTagFromDesign,
  revealInFolder,
  setInkstitchConfig,
  unlinkArtworkFromDesign,
} from "../lib";
import type {
  ArtworkAsset,
  Collection,
  Design,
  DesignDetails,
  Job,
} from "../types";
import { DesignImage } from "./DesignImage";

interface DesignDetailsDrawerProps {
  designId: string;
  onClose: () => void;
  onRefreshCatalog: () => void;
  onTriggerAi: (design: Design) => void;
  onDeleteDesign: (id: string) => void;
  onRestoreDesign?: (id: string) => void;
}

const SUPPORTED_EXPORTS = [
  "DST",
  "PES",
  "JEF",
  "VP3",
  "EXP",
  "XXX",
  "PEC",
];

export const DesignDetailsDrawer: React.FC<DesignDetailsDrawerProps> = ({
  designId,
  onClose,
  onRefreshCatalog,
  onTriggerAi,
  onDeleteDesign,
  onRestoreDesign,
}) => {
  const [details, setDetails] = useState<DesignDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const [newTagInput, setNewTagInput] = useState("");
  const [copiedChecksum, setCopiedChecksum] = useState(false);
  const [exportFormat, setExportFormat] = useState("DST");
  const [isExporting, setIsExporting] = useState(false);
  const [exportNotice, setExportNotice] = useState<string | null>(null);

  // Pickers state
  const [availableCollections, setAvailableCollections] = useState<Collection[]>([]);
  const [availableJobs, setAvailableJobs] = useState<Job[]>([]);
  const [availableArtwork, setAvailableArtwork] = useState<ArtworkAsset[]>([]);
  const [showArtworkPicker, setShowArtworkPicker] = useState(false);

  const loadData = async (targetId: string) => {
    try {
      setLoading(true);
      setExportNotice(null);
      setNewTagInput("");
      setShowArtworkPicker(false);

      const [d, cols, jobs, arts] = await Promise.all([
        getDesignDetails(targetId),
        listCollections(),
        listJobs(),
        listArtwork(),
      ]);
      setDetails(d);
      setAvailableCollections(cols);
      setAvailableJobs(jobs);
      setAvailableArtwork(arts);
    } catch (err) {
      console.error("Failed to load design details:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    setDetails(null);
    void loadData(designId);
  }, [designId]);

  if (!details && loading) {
    return (
      <aside className="details-drawer">
        <div className="drawer-loading">
          <RefreshCw size={24} className="spin text-accent" />
          <p>Loading design details…</p>
        </div>
      </aside>
    );
  }

  if (!details) return null;
  const d = details.design;

  const handleCopyChecksum = (hash: string) => {
    void navigator.clipboard.writeText(hash);
    setCopiedChecksum(true);
    setTimeout(() => setCopiedChecksum(false), 2000);
  };

  const handleAddTag = async (e: React.FormEvent) => {
    e.preventDefault();
    const tag = newTagInput.trim();
    if (!tag) return;
    try {
      await addTagToDesign(d.id, tag);
      setNewTagInput("");
      await loadData(d.id);
      onRefreshCatalog();
    } catch (err) {
      console.error(err);
    }
  };

  const handleRemoveTag = async (tag: string) => {
    try {
      await removeTagFromDesign(d.id, tag);
      await loadData(d.id);
      onRefreshCatalog();
    } catch (err) {
      console.error(err);
    }
  };

  const handleSetCollection = async (collectionId: string) => {
    try {
      if (d.collectionId) {
        await removeDesignFromCollection(d.collectionId, d.id);
      }
      if (collectionId) {
        await addDesignToCollection(collectionId, d.id);
      }
      await loadData(d.id);
      onRefreshCatalog();
    } catch (err) {
      console.error(err);
    }
  };

  const handleSetJob = async (jobId: string) => {
    try {
      if (d.jobId) {
        await removeDesignFromJob(d.jobId, d.id);
      }
      if (jobId) {
        await addDesignToJob(jobId, d.id);
      }
      await loadData(d.id);
      onRefreshCatalog();
    } catch (err) {
      console.error(err);
    }
  };

  const handleLinkArtwork = async (assetId: string) => {
    try {
      await linkArtworkToDesign(d.id, assetId);
      setShowArtworkPicker(false);
      await loadData(d.id);
    } catch (err) {
      console.error(err);
    }
  };

  const handleUnlinkArtwork = async (assetId: string) => {
    try {
      await unlinkArtworkFromDesign(d.id, assetId);
      await loadData(d.id);
    } catch (err) {
      console.error(err);
    }
  };

  const handleExport = async () => {
    try {
      setIsExporting(true);
      setExportNotice(null);
      const suggestedName = `${d.title.toLowerCase().replace(/\s+/g, "_")}.${exportFormat.toLowerCase()}`;
      const filePath = await save({
        defaultPath: suggestedName,
        filters: [{ name: `${exportFormat} Embroidery File`, extensions: [exportFormat.toLowerCase()] }],
      });

      if (!filePath) {
        setIsExporting(false);
        return;
      }

      await exportDesign(d.id, filePath, exportFormat);
      setExportNotice(`Exported successfully to ${filePath.split(/[\\/]/).pop()}`);
    } catch (err) {
      setExportNotice(formatError(err, "Export failed"));
    } finally {
      setIsExporting(false);
    }
  };

  const handleReveal = () => {
    if (d.managedPath) {
      void revealInFolder(d.managedPath);
    }
  };

  const handleInkstitch = async () => {
    try {
      await openInInkstitch(d.id);
    } catch (err) {
      const msg = formatError(err);
      if (
        msg.toLowerCase().includes("not configured") ||
        msg.toLowerCase().includes("path") ||
        msg.toLowerCase().includes("executable")
      ) {
        if (
          confirm(
            "Inkscape executable path is not configured. Would you like to select inkscape.exe now?"
          )
        ) {
          try {
            const file = await open({
              multiple: false,
              title: "Select Inkscape Executable (inkscape.exe)",
              filters: [{ name: "Executable", extensions: ["exe"] }],
            });
            if (file && typeof file === "string") {
              await setInkstitchConfig(file);
              await openInInkstitch(d.id);
            }
          } catch (configErr) {
            alert(formatError(configErr, "Failed to set Inkscape path"));
          }
        }
      } else {
        alert(msg);
      }
    }
  };

  const isRecycled = d.status === "recycled";

  return (
    <aside className="details-drawer">
      <div className="drawer-header">
        <div>
          <span className="eyebrow">DESIGN INSPECTOR</span>
          <h2>{d.title}</h2>
          <span className="subtle text-xs">{d.filename}</span>
        </div>
        <button className="icon-button" onClick={onClose} aria-label="Close">
          <X size={19} />
        </button>
      </div>

      <div className="drawer-body">
        {/* Rendered Preview Banner */}
        <div className="drawer-preview-box">
          <DesignImage
            previewPath={d.previewPath}
            title={d.title}
            format={d.format}
            large={true}
          />
        </div>

        {/* Quick Action Toolbar */}
        <div className="drawer-action-row">
          <button
            className="secondary flex-1"
            onClick={() => onTriggerAi(d)}
            title="Analyze preview with Vision AI"
          >
            <Sparkles size={16} className="text-accent" />
            <span>Analyze with AI</span>
          </button>
          <button
            className="secondary icon-btn"
            onClick={handleReveal}
            title="Reveal file in Windows Explorer"
          >
            <FolderOpen size={16} />
          </button>
          <button
            className="primary icon-btn"
            onClick={handleInkstitch}
            title="Open in Ink/Stitch (Inkscape)"
          >
            <ExternalLink size={16} />
          </button>
        </div>

        {/* Technical Facts Grid */}
        <section className="drawer-section">
          <h4 className="section-title">TECHNICAL FACTS</h4>
          <div className="facts-grid">
            <div className="fact-item">
              <span className="fact-label">Format</span>
              <b className="fact-val format-highlight">{d.format}</b>
            </div>
            <div className="fact-item">
              <span className="fact-label">Dimensions</span>
              <b className="fact-val">
                {d.widthMm ? `${d.widthMm.toFixed(1)} × ${d.heightMm?.toFixed(1)} mm` : "—"}
              </b>
            </div>
            <div className="fact-item">
              <span className="fact-label">Stitches</span>
              <b className="fact-val">{d.stitches?.toLocaleString() ?? "—"}</b>
            </div>
            <div className="fact-item">
              <span className="fact-label">Colors</span>
              <b className="fact-val">{d.colors ?? "—"}</b>
            </div>
            <div className="fact-item">
              <span className="fact-label">File Size</span>
              <b className="fact-val">{(d.sizeBytes / 1024).toFixed(1)} KB</b>
            </div>
            <div className="fact-item">
              <span className="fact-label">Imported</span>
              <b className="fact-val text-xs">
                {new Date(d.importedAt).toLocaleDateString()}
              </b>
            </div>
          </div>
        </section>

        {/* Thread Color Sequence */}
        {d.threads && d.threads.length > 0 && (
          <section className="drawer-section">
            <h4 className="section-title">THREAD PALETTE ({d.threads.length})</h4>
            <div className="thread-palette-list">
              {d.threads.map((th) => (
                <div key={th.index} className="thread-item">
                  <span
                    className="thread-swatch"
                    style={{ backgroundColor: th.hex }}
                  />
                  <div className="thread-details">
                    <span className="thread-num">#{th.index}</span>
                    <span className="thread-desc">{th.description || th.hex}</span>
                    {th.brand && <small className="thread-brand">{th.brand}</small>}
                  </div>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* AI Suggestions / Metadata */}
        {(d.aiCategory || d.aiDescription) && (
          <section className="drawer-section ai-insights-card">
            <h4 className="section-title text-accent">
              <Sparkles size={14} /> AI CLASSIFICATIONS
            </h4>
            {d.aiCategory && (
              <p className="insight-row">
                <b>Category:</b> {d.aiCategory} {d.aiSubject && `· ${d.aiSubject}`}
              </p>
            )}
            {d.aiStyle && (
              <p className="insight-row">
                <b>Style:</b> {d.aiStyle}
              </p>
            )}
            {d.aiDescription && (
              <p className="insight-desc">{d.aiDescription}</p>
            )}
            {d.dominantColors && d.dominantColors.length > 0 && (
              <div className="dom-colors-row">
                <small>Dominant Palette:</small>
                {d.dominantColors.map((c, i) => (
                  <span
                    key={i}
                    className="color-chip"
                    style={{ backgroundColor: c }}
                    title={c}
                  />
                ))}
              </div>
            )}
          </section>
        )}

        {/* Ink/Stitch Handoff Section */}
        <section className="drawer-section">
          <div className="section-head-row">
            <h4 className="section-title">
              <ExternalLink size={14} /> INK/STITCH INTEGRATION
            </h4>
          </div>
          <p className="text-xs text-subtle mb-3">
            Open this design directly inside Inkscape with the Ink/Stitch extension for vector editing and simulation.
          </p>
          <button
            className="secondary full-width"
            type="button"
            onClick={handleInkstitch}
          >
            <ExternalLink size={15} />
            <span>Open in Ink/Stitch (Inkscape)</span>
          </button>
        </section>

        {/* Tags Management */}
        <section className="drawer-section">
          <div className="section-head-row">
            <h4 className="section-title">TAGS</h4>
          </div>
          <div className="tag-badges-container">
            {d.tags.map((t) => (
              <span key={t} className="tag-badge-editable">
                #{t}
                <button
                  className="tag-remove-btn"
                  onClick={() => handleRemoveTag(t)}
                  title="Remove tag"
                >
                  <X size={12} />
                </button>
              </span>
            ))}
            {d.tags.length === 0 && (
              <span className="empty-subtle">No tags assigned yet.</span>
            )}
          </div>
          <form onSubmit={handleAddTag} className="inline-add-form mt-2">
            <input
              type="text"
              placeholder="Add tag (e.g. floral, cap, logo)…"
              value={newTagInput}
              onChange={(e) => setNewTagInput(e.target.value)}
              className="compact-input"
            />
            <button type="submit" className="secondary compact-btn">
              Add
            </button>
          </form>
        </section>

        {/* Organization: Collections & Jobs */}
        <section className="drawer-section">
          <h4 className="section-title">ORGANIZATION</h4>
          <div className="org-assignment-fields">
            <div className="org-field">
              <label>Collection:</label>
              <select
                value={d.collectionId || ""}
                onChange={(e) => handleSetCollection(e.target.value)}
                className="select-input"
              >
                <option value="">None (Unassigned)</option>
                {availableCollections.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
            </div>

            <div className="org-field">
              <label>Job / Production Batch:</label>
              <select
                value={d.jobId || ""}
                onChange={(e) => handleSetJob(e.target.value)}
                className="select-input"
              >
                <option value="">None (Unassigned)</option>
                {availableJobs.map((j) => (
                  <option key={j.id} value={j.id}>
                    {j.title} ({j.status})
                  </option>
                ))}
              </select>
            </div>
          </div>
        </section>

        {/* Linked Source Artwork */}
        <section className="drawer-section">
          <div className="section-head-row">
            <h4 className="section-title">LINKED SOURCE ARTWORK</h4>
            <button
              className="text-button text-xs"
              onClick={() => setShowArtworkPicker(!showArtworkPicker)}
            >
              {showArtworkPicker ? "Cancel" : "+ Link Artwork"}
            </button>
          </div>

          {showArtworkPicker && (
            <div className="artwork-picker-dropdown">
              <p className="text-xs text-subtle mb-2">
                Select an imported artwork asset to link:
              </p>
              <div className="artwork-picker-list">
                {availableArtwork
                  .filter(
                    (art) => !details.linkedArtwork.some((la) => la.id === art.id)
                  )
                  .map((art) => (
                    <button
                      key={art.id}
                      className="artwork-picker-item"
                      onClick={() => handleLinkArtwork(art.id)}
                    >
                      <ImageIcon size={14} />
                      <span>{art.filename}</span>
                    </button>
                  ))}
              </div>
            </div>
          )}

          {details.linkedArtwork.length === 0 ? (
            <p className="empty-subtle">No source artwork linked.</p>
          ) : (
            <div className="linked-artwork-list">
              {details.linkedArtwork.map((art) => (
                <div key={art.id} className="linked-artwork-item">
                  <ImageIcon size={16} />
                  <div className="flex-1 min-w-0">
                    <b className="truncate block text-sm">{art.filename}</b>
                    <span className="text-xs text-subtle">{art.mimeType}</span>
                  </div>
                  <button
                    className="icon-button-sm"
                    onClick={() => handleUnlinkArtwork(art.id)}
                    title="Unlink artwork"
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>

        {/* File Revisions */}
        <section className="drawer-section">
          <h4 className="section-title">
            <History size={14} /> REVISION HISTORY ({details.revisions.length})
          </h4>
          <div className="revision-timeline">
            {details.revisions.map((rev) => (
              <div key={rev.id} className="revision-entry">
                <span className="rev-number">v{rev.revisionNumber}</span>
                <div className="rev-info">
                  <b>{rev.note || "Revision update"}</b>
                  <span className="text-xs text-subtle">
                    {new Date(rev.createdAt).toLocaleString()} · {(rev.sizeBytes / 1024).toFixed(1)} KB
                  </span>
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* Export Conversion Section */}
        <section className="drawer-section export-section">
          <h4 className="section-title">CONVERT & EXPORT</h4>
          <div className="export-controls-row">
            <select
              value={exportFormat}
              onChange={(e) => setExportFormat(e.target.value)}
              className="select-input compact-select"
            >
              {SUPPORTED_EXPORTS.map((fmt) => (
                <option key={fmt} value={fmt}>
                  Convert to {fmt}
                </option>
              ))}
            </select>
            <button
              className="primary compact-btn"
              onClick={handleExport}
              disabled={isExporting}
            >
              <FileDown size={15} />
              <span>{isExporting ? "Exporting…" : "Export File"}</span>
            </button>
          </div>
          {exportNotice && (
            <p className="notice-text text-xs mt-2">{exportNotice}</p>
          )}
        </section>

        {/* File Provenance & Checksum */}
        <section className="drawer-section provenance-section">
          <div className="provenance-row">
            <small>SHA-256 Checksum:</small>
            <button
              className="copy-hash-btn"
              onClick={() => handleCopyChecksum(details.revisions[0]?.checksum || "")}
              title="Copy checksum"
            >
              <code className="text-xs">
                {details.revisions[0]?.checksum.slice(0, 16)}…
              </code>
              {copiedChecksum ? <Check size={12} /> : <Copy size={12} />}
            </button>
          </div>
        </section>
      </div>

      {/* Drawer Footer Actions */}
      <div className="drawer-footer">
        {isRecycled ? (
          <>
            <button
              className="secondary flex-1"
              onClick={() => onRestoreDesign?.(d.id)}
            >
              <Undo2 size={16} /> Restore Design
            </button>
            <button
              className="danger flex-1"
              onClick={() => {
                if (confirm("Permanently delete this design? This cannot be undone.")) {
                  void permanentDeleteDesign(d.id).then(() => {
                    onClose();
                    onRefreshCatalog();
                  });
                }
              }}
            >
              <Trash2 size={16} /> Delete Forever
            </button>
          </>
        ) : (
          <button
            className="danger full-width"
            onClick={() => onDeleteDesign(d.id)}
          >
            <Trash2 size={16} /> Move to Recycle Area
          </button>
        )}
      </div>
    </aside>
  );
};
