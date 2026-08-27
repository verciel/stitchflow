import React, { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  CheckCircle2,
  Copy,
  FileCode2,
  FolderOpen,
  HelpCircle,
  Loader2,
  Upload,
  X,
} from "lucide-react";
import { importFiles } from "../lib";
import type { ImportResult } from "../types";

interface BatchImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onImportComplete: () => void;
}

export const BatchImportModal: React.FC<BatchImportModalProps> = ({
  isOpen,
  onClose,
  onImportComplete,
}) => {
  const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
  const [duplicatePolicy, setDuplicatePolicy] = useState<
    "skip" | "keep_both" | "replace_revision"
  >("skip");
  const [isProcessing, setIsProcessing] = useState(false);
  const [results, setResults] = useState<ImportResult[] | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  if (!isOpen) return null;

  const handlePickFiles = async () => {
    try {
      const selection = await open({
        multiple: true,
        filters: [
          {
            name: "Supported files (Embroidery & Artwork)",
            extensions: [
              "dst",
              "pes",
              "jef",
              "vp3",
              "exp",
              "hus",
              "xxx",
              "sew",
              "pcs",
              "pec",
              "png",
              "jpg",
              "jpeg",
              "svg",
              "pdf",
            ],
          },
        ],
      });

      if (!selection) return;
      const paths = Array.isArray(selection) ? selection : [selection];
      setSelectedPaths(paths);
      setResults(null);
      setErrorMsg(null);
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : "Failed to open file picker");
    }
  };

  const handleRunImport = async () => {
    if (!selectedPaths.length) return;
    setIsProcessing(true);
    setErrorMsg(null);

    try {
      const importResults = await importFiles(selectedPaths, duplicatePolicy);
      setResults(importResults);
      onImportComplete();
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : "Import failed");
    } finally {
      setIsProcessing(false);
    }
  };

  const handleReset = () => {
    setSelectedPaths([]);
    setResults(null);
    setErrorMsg(null);
  };

  const importedCount = results?.filter((r) => r.status === "imported").length ?? 0;
  const duplicateCount = results?.filter((r) => r.status === "duplicate").length ?? 0;
  const errorCount =
    results?.filter((r) => r.status === "unsupported" || r.status === "failed" || r.status === "invalid")
      .length ?? 0;

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal import-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-group">
            <Upload size={22} className="modal-icon-accent" />
            <div>
              <h2>Batch Import Designs & Artwork</h2>
              <p className="subtle">
                Import DST, PES, JEF, VP3, EXP, HUS, XXX, SEW, PCS, PEC, and artwork assets (PNG, JPG, SVG, PDF).
              </p>
            </div>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          {errorMsg && (
            <div className="alert-banner alert-error">
              <AlertCircle size={18} />
              <span>{errorMsg}</span>
            </div>
          )}

          {!results && (
            <>
              {selectedPaths.length === 0 ? (
                <div className="dropzone-area" onClick={handlePickFiles}>
                  <div className="dropzone-icon">
                    <FolderOpen size={36} />
                  </div>
                  <h3>Select files or folder for batch import</h3>
                  <p>Original files remain untouched. Stitchflow creates managed copies with checksum validation.</p>
                  <button className="primary mt-3" type="button" onClick={handlePickFiles}>
                    Browse Files…
                  </button>
                </div>
              ) : (
                <div className="file-selection-preview">
                  <div className="selection-header">
                    <span>
                      <b>{selectedPaths.length}</b> file(s) selected
                    </span>
                    <button className="text-button text-sm" onClick={handleReset}>
                      Change Selection
                    </button>
                  </div>
                  <ul className="file-path-list">
                    {selectedPaths.slice(0, 8).map((p, idx) => (
                      <li key={idx} className="file-path-item">
                        <FileCode2 size={16} />
                        <span className="file-name">{p.split(/[\\/]/).pop()}</span>
                        <span className="file-dir">{p}</span>
                      </li>
                    ))}
                    {selectedPaths.length > 8 && (
                      <li className="file-path-more">
                        + {selectedPaths.length - 8} more files
                      </li>
                    )}
                  </ul>
                </div>
              )}

              <div className="duplicate-policy-section">
                <label className="section-label">
                  EXACT DUPLICATE HANDLING (SHA-256 MATCH)
                </label>
                <div className="policy-options-grid">
                  <label className={`policy-card ${duplicatePolicy === "skip" ? "active" : ""}`}>
                    <input
                      type="radio"
                      name="dupPolicy"
                      value="skip"
                      checked={duplicatePolicy === "skip"}
                      onChange={() => setDuplicatePolicy("skip")}
                    />
                    <div>
                      <b>Skip duplicates</b>
                      <p>Ignore exact checksum matches to prevent redundant entries.</p>
                    </div>
                  </label>

                  <label className={`policy-card ${duplicatePolicy === "replace_revision" ? "active" : ""}`}>
                    <input
                      type="radio"
                      name="dupPolicy"
                      value="replace_revision"
                      checked={duplicatePolicy === "replace_revision"}
                      onChange={() => setDuplicatePolicy("replace_revision")}
                    />
                    <div>
                      <b>Replace as revision</b>
                      <p>Attach file as a new immutable version under the existing design.</p>
                    </div>
                  </label>

                  <label className={`policy-card ${duplicatePolicy === "keep_both" ? "active" : ""}`}>
                    <input
                      type="radio"
                      name="dupPolicy"
                      value="keep_both"
                      checked={duplicatePolicy === "keep_both"}
                      onChange={() => setDuplicatePolicy("keep_both")}
                    />
                    <div>
                      <b>Keep both</b>
                      <p>Import as a distinct catalog item and flag it as a duplicate.</p>
                    </div>
                  </label>
                </div>
              </div>
            </>
          )}

          {isProcessing && (
            <div className="import-progress-box">
              <Loader2 size={32} className="spin text-accent" />
              <h3>Extracting metadata and rendering stitch previews…</h3>
              <p>Analyzing stitch paths, thread sequences, and hoop coordinates deterministically.</p>
            </div>
          )}

          {results && (
            <div className="import-results-container">
              <div className="results-summary-bar">
                <div className="summary-pill success">
                  <CheckCircle2 size={16} />
                  <span>{importedCount} imported</span>
                </div>
                {duplicateCount > 0 && (
                  <div className="summary-pill duplicate">
                    <Copy size={16} />
                    <span>{duplicateCount} duplicate(s)</span>
                  </div>
                )}
                {errorCount > 0 && (
                  <div className="summary-pill error">
                    <AlertCircle size={16} />
                    <span>{errorCount} skipped / invalid</span>
                  </div>
                )}
              </div>

              <div className="results-table-scroll">
                <table className="results-table">
                  <thead>
                    <tr>
                      <th>File</th>
                      <th>Status</th>
                      <th>Details</th>
                    </tr>
                  </thead>
                  <tbody>
                    {results.map((res, i) => (
                      <tr key={i} className={`row-${res.status}`}>
                        <td className="font-mono text-sm">
                          {res.path.split(/[\\/]/).pop()}
                        </td>
                        <td>
                          <span className={`status-badge-inline ${res.status}`}>
                            {res.status}
                          </span>
                        </td>
                        <td className="text-sm text-subtle">
                          {res.message || (res.design ? `${res.design.format} · ${res.design.stitches?.toLocaleString()} sts` : "—")}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>

        <div className="modal-footer">
          {results ? (
            <button className="primary" onClick={onClose}>
              Done
            </button>
          ) : (
            <>
              <button className="text-button" onClick={onClose} disabled={isProcessing}>
                Cancel
              </button>
              <button
                className="primary"
                onClick={handleRunImport}
                disabled={selectedPaths.length === 0 || isProcessing}
              >
                {isProcessing ? "Importing…" : `Import ${selectedPaths.length || ""} Files`}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
