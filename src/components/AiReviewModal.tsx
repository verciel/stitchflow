import React, { useEffect, useState } from "react";
import {
  AlertCircle,
  Bot,
  Check,
  CheckCircle2,
  RefreshCw,
  ShieldCheck,
  Sparkles,
  Tag as TagIcon,
  X,
} from "lucide-react";
import {
  analyzeDesigns,
  applyAiSuggestion,
  formatError,
  getAiConfig,
  saveAiConfig,
} from "../lib";
import type { AiConfig, AiSuggestion, Design } from "../types";
import { DesignImage } from "./DesignImage";

interface AiReviewModalProps {
  design: Design | null;
  aiConfig: AiConfig;
  isOpen: boolean;
  onClose: () => void;
  onApplied: () => void;
  onOpenSettings?: () => void;
}

export const AiReviewModal: React.FC<AiReviewModalProps> = ({
  design,
  aiConfig: initialConfig,
  isOpen,
  onClose,
  onApplied,
  onOpenSettings,
}) => {
  const [currentConfig, setCurrentConfig] = useState<AiConfig>(initialConfig);
  const [analyzing, setAnalyzing] = useState(false);
  const [suggestion, setSuggestion] = useState<AiSuggestion | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appliedNotice, setAppliedNotice] = useState(false);

  // Always refresh latest AI config from database when modal opens
  useEffect(() => {
    if (isOpen) {
      setError(null);
      setSuggestion(null);
      setAppliedNotice(false);
      void getAiConfig().then((conf) => {
        // If an API key is present or local endpoint is used, ensure it is treated as enabled
        if (conf.apiKey.trim() || !conf.endpoint.includes("api.openai.com")) {
          if (!conf.enabled) {
            const enabledConf = { ...conf, enabled: true };
            void saveAiConfig(enabledConf);
            setCurrentConfig(enabledConf);
            return;
          }
        }
        setCurrentConfig(conf);
      });
    }
  }, [isOpen]);

  if (!isOpen || !design) return null;

  const handleEnableAi = async () => {
    try {
      const updated = { ...currentConfig, enabled: true };
      await saveAiConfig(updated);
      setCurrentConfig(updated);
    } catch (err) {
      setError(formatError(err, "Failed to enable AI"));
    }
  };

  const handleStartAnalysis = async () => {
    try {
      setAnalyzing(true);
      setError(null);

      // Auto-enable if key is present
      if (!currentConfig.enabled) {
        await handleEnableAi();
      }

      const results = await analyzeDesigns([design.id]);
      if (results.length > 0) {
        setSuggestion(results[0]);
      } else {
        setError("AI returned an empty suggestion.");
      }
    } catch (err) {
      setError(formatError(err, "AI analysis request failed"));
    } finally {
      setAnalyzing(false);
    }
  };

  const handleApply = async (accepted: boolean) => {
    if (!suggestion) return;
    try {
      await applyAiSuggestion(suggestion.id, accepted);
      if (accepted) {
        setAppliedNotice(true);
        setTimeout(() => {
          onApplied();
          onClose();
        }, 1200);
      } else {
        onApplied();
        onClose();
      }
    } catch (err) {
      setError(formatError(err, "Failed to apply suggestion"));
    }
  };

  const isOpenAiMissingKey =
    currentConfig.endpoint.includes("api.openai.com") && !currentConfig.apiKey.trim();

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal ai-review-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-group">
            <Sparkles size={22} className="text-accent" />
            <div>
              <h2>AI Vision Design Analysis</h2>
              <span className="subtle text-xs">
                Target: {design.title} ({design.format})
              </span>
            </div>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>

        <div className="modal-body">
          {error && (
            <div className="alert-banner alert-error mb-3">
              <AlertCircle size={16} />
              <span>{error}</span>
            </div>
          )}

          {appliedNotice ? (
            <div className="ai-applied-success">
              <CheckCircle2 size={42} className="text-green-500" />
              <h3>Suggestions applied to design catalog!</h3>
              <p>Metadata, description, and tags updated successfully.</p>
            </div>
          ) : !suggestion && !analyzing ? (
            <div className="ai-consent-panel">
              {!currentConfig.enabled && !isOpenAiMissingKey && (
                <div className="alert-banner alert-warning mb-3">
                  <AlertCircle size={18} />
                  <div style={{ flex: 1 }}>
                    <b>AI Features are Paused</b>
                    <p className="text-xs mt-1">
                      Click below to activate AI Vision analysis for this catalog.
                    </p>
                  </div>
                  <button
                    type="button"
                    className="primary compact-btn"
                    onClick={handleEnableAi}
                  >
                    Enable AI Now
                  </button>
                </div>
              )}

              {isOpenAiMissingKey && (
                <div className="alert-banner alert-error mb-3">
                  <AlertCircle size={18} />
                  <div style={{ flex: 1 }}>
                    <b>OpenAI API Key Missing</b>
                    <p className="text-xs mt-1">
                      Please enter your OpenAI API key in Settings, or configure a local server like Ollama or LM Studio.
                    </p>
                  </div>
                  {onOpenSettings && (
                    <button
                      type="button"
                      className="secondary compact-btn"
                      onClick={onOpenSettings}
                    >
                      Open Settings
                    </button>
                  )}
                </div>
              )}

              <div className="ai-target-summary">
                <DesignImage
                  previewPath={design.previewPath}
                  title={design.title}
                  format={design.format}
                />
                <div className="ai-target-facts">
                  <b>{design.title}</b>
                  <p className="text-xs text-subtle">{design.filename}</p>
                  <div className="text-xs mt-2">
                    <span>{design.stitches?.toLocaleString() ?? 0} stitches</span> ·{" "}
                    <span>
                      {design.widthMm?.toFixed(1) ?? "—"} × {design.heightMm?.toFixed(1) ?? "—"} mm
                    </span>
                  </div>
                </div>
              </div>

              <div className="privacy-card-banner">
                <ShieldCheck size={20} className="text-green-500" />
                <div>
                  <b>Privacy & Data Boundary</b>
                  <p className="text-xs text-subtle mt-1">
                    Stitchflow will send <b>only</b> the rendered 2D preview image and approved technical facts (stitches, dimensions, colors). The original embroidery file will <b>never</b> be uploaded.
                  </p>
                </div>
              </div>

              <div className="provider-spec-box">
                <Bot size={16} />
                <span>
                  Provider: <b>{currentConfig.endpoint}</b> (Model: <b>{currentConfig.model}</b>)
                </span>
              </div>
            </div>
          ) : analyzing ? (
            <div className="ai-analyzing-box">
              <RefreshCw size={36} className="spin text-accent" />
              <h3>Analyzing stitch pattern with Vision API…</h3>
              <p className="subtle text-xs">
                Generating high-fidelity catalog classifications, proposed tags, and thread color insights.
              </p>
            </div>
          ) : (
            <div className="ai-suggestion-review-grid">
              <div className="suggestion-preview-col">
                <DesignImage
                  previewPath={design.previewPath}
                  title={design.title}
                  format={design.format}
                />
                <div className="confidence-pill mt-3">
                  <span>Confidence:</span>
                  <b>{Math.round((suggestion?.confidence ?? 0.9) * 100)}%</b>
                </div>
              </div>

              <div className="suggestion-details-col">
                <div className="suggestion-field">
                  <label>PROPOSED CATEGORY & SUBJECT</label>
                  <p className="field-value font-medium">
                    {suggestion?.category || "Uncategorized"}
                    {suggestion?.subject ? ` — ${suggestion.subject}` : ""}
                  </p>
                </div>

                {suggestion?.style && (
                  <div className="suggestion-field">
                    <label>EMBROIDERY STYLE</label>
                    <p className="field-value">{suggestion.style}</p>
                  </div>
                )}

                {suggestion?.description && (
                  <div className="suggestion-field">
                    <label>SUGGESTED DESCRIPTION</label>
                    <p className="field-value text-sm italic">
                      "{suggestion.description}"
                    </p>
                  </div>
                )}

                {suggestion?.tags && suggestion.tags.length > 0 && (
                  <div className="suggestion-field">
                    <label>PROPOSED TAGS</label>
                    <div className="tag-badges-container mt-1">
                      {suggestion.tags.map((t) => (
                        <span key={t} className="tag-badge">
                          #{t}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {suggestion?.dominantColors && suggestion.dominantColors.length > 0 && (
                  <div className="suggestion-field">
                    <label>DOMINANT PALETTE</label>
                    <div className="dom-colors-row mt-1">
                      {suggestion.dominantColors.map((c, i) => (
                        <span
                          key={i}
                          className="color-chip"
                          style={{ backgroundColor: c }}
                          title={c}
                        />
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="modal-footer">
          {!suggestion && !analyzing && !appliedNotice && (
            <>
              <button className="text-button" onClick={onClose}>
                Cancel
              </button>
              <button
                className="primary"
                onClick={handleStartAnalysis}
                disabled={isOpenAiMissingKey}
              >
                <Sparkles size={16} /> Analyze Design
              </button>
            </>
          )}

          {suggestion && !appliedNotice && (
            <div className="suggestion-footer-actions">
              <button
                className="secondary"
                onClick={() => handleApply(false)}
              >
                Dismiss
              </button>
              <button
                className="primary"
                onClick={() => handleApply(true)}
              >
                <Check size={16} /> Accept & Apply to Catalog
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
