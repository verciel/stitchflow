import React, { useEffect, useState } from "react";
import {
  AlertCircle,
  Bot,
  Check,
  CheckCircle2,
  HelpCircle,
  MessageSquare,
  RefreshCw,
  Send,
  ShieldCheck,
  Sparkles,
  Tag as TagIcon,
  X,
} from "lucide-react";
import {
  analyzeDesigns,
  applyAiSuggestion,
  askAiCustom,
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
  const [activeTab, setActiveTab] = useState<"catalog" | "chat">("catalog");
  const [currentConfig, setCurrentConfig] = useState<AiConfig>(initialConfig);
  const [analyzing, setAnalyzing] = useState(false);
  const [suggestion, setSuggestion] = useState<AiSuggestion | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appliedNotice, setAppliedNotice] = useState(false);

  // Custom Q&A State
  const [customQuestion, setCustomQuestion] = useState("");
  const [askingAi, setAskingAi] = useState(false);
  const [aiAnswer, setAiAnswer] = useState<string | null>(null);

  // Always refresh latest AI config from database when modal opens
  useEffect(() => {
    if (isOpen) {
      setError(null);
      setSuggestion(null);
      setAppliedNotice(false);
      setAiAnswer(null);
      setCustomQuestion("");
      void getAiConfig().then((conf) => {
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

  const handleAskQuestion = async (promptText?: string) => {
    const q = (promptText || customQuestion).trim();
    if (!q) return;
    try {
      setAskingAi(true);
      setError(null);
      if (promptText) setCustomQuestion(promptText);

      const ans = await askAiCustom(design.id, q);
      setAiAnswer(ans);
    } catch (err) {
      setError(formatError(err, "Failed to get AI advice"));
    } finally {
      setAskingAi(false);
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
      <div className="modal ai-review-modal" style={{ maxWidth: "740px" }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-group">
            <Sparkles size={22} className="text-accent" />
            <div>
              <h2>AI Embroidery Assistant</h2>
              <span className="subtle text-xs">
                Target: {design.title} ({design.format} · {design.stitches?.toLocaleString() ?? 0} stitches)
              </span>
            </div>
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>

        {/* Modal Tab Switcher */}
        <div className="modal-tabs" style={{ display: "flex", gap: "8px", padding: "12px 24px 0", borderBottom: "1px solid var(--border-color)" }}>
          <button
            type="button"
            className={`tab-btn ${activeTab === "catalog" ? "active font-bold text-accent" : "subtle"}`}
            style={{ padding: "8px 16px", borderBottom: activeTab === "catalog" ? "2px solid var(--accent)" : "none", background: "none", cursor: "pointer" }}
            onClick={() => setActiveTab("catalog")}
          >
            ✨ Catalog Tagging & Vision
          </button>
          <button
            type="button"
            className={`tab-btn ${activeTab === "chat" ? "active font-bold text-accent" : "subtle"}`}
            style={{ padding: "8px 16px", borderBottom: activeTab === "chat" ? "2px solid var(--accent)" : "none", background: "none", cursor: "pointer" }}
            onClick={() => setActiveTab("chat")}
          >
            💬 Production Advice & Changes
          </button>
        </div>

        <div className="modal-body" style={{ maxHeight: "65vh", overflowY: "auto", padding: "20px 24px" }}>
          {error && (
            <div className="alert-banner alert-error mb-3">
              <AlertCircle size={16} />
              <span>{error}</span>
            </div>
          )}

          {activeTab === "catalog" ? (
            /* Catalog Tagging & Vision Tab */
            appliedNotice ? (
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
                        Click below to activate AI analysis for this catalog.
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
                      <b>API Key Missing</b>
                      <p className="text-xs mt-1">
                        Please enter your API key (OpenAI or Groq) in Settings.
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
                    <b>What does analysis do?</b>
                    <p className="text-xs text-subtle mt-1">
                      Stitchflow analyzes technical stitch patterns to propose <b>search tags</b>, <b>subject categories</b>, <b>color palettes</b>, and <b>catalog summaries</b> so you can find designs instantly via natural language search.
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
                <h3>Analyzing stitch pattern with AI…</h3>
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
                      <label>PROPOSED SEARCH TAGS</label>
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
            )
          ) : (
            /* Interactive Q&A / Production Advice Tab */
            <div className="ai-chat-panel">
              <div className="chat-prompt-quick-chips" style={{ marginBottom: "16px" }}>
                <span className="text-xs font-bold text-subtle" style={{ display: "block", marginBottom: "8px" }}>
                  💡 QUICK PRODUCTION PROMPTS:
                </span>
                <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
                  {[
                    "Recommended backing & needles for pique polo shirts?",
                    "Can this design be adapted for baseball caps / curved hoops?",
                    "How to recolor this design for black/dark garments?",
                    "Suggest matching companion design ideas for this theme",
                  ].map((chip) => (
                    <button
                      key={chip}
                      type="button"
                      className="secondary compact-btn"
                      style={{ fontSize: "11px", padding: "4px 8px" }}
                      onClick={() => handleAskQuestion(chip)}
                    >
                      {chip}
                    </button>
                  ))}
                </div>
              </div>

              <div className="chat-input-row" style={{ display: "flex", gap: "8px", marginBottom: "16px" }}>
                <input
                  type="text"
                  placeholder="Ask anything about modifying this design, fabric recipes, thread matching…"
                  value={customQuestion}
                  onChange={(e) => setCustomQuestion(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !askingAi) handleAskQuestion();
                  }}
                  style={{ flex: 1 }}
                />
                <button
                  type="button"
                  className="primary"
                  onClick={() => handleAskQuestion()}
                  disabled={askingAi || !customQuestion.trim() || isOpenAiMissingKey}
                >
                  {askingAi ? <RefreshCw size={16} className="spin" /> : <Send size={16} />}
                  <span>Ask</span>
                </button>
              </div>

              {askingAi && (
                <div className="ai-analyzing-box" style={{ padding: "24px" }}>
                  <RefreshCw size={28} className="spin text-accent" />
                  <p className="subtle text-xs mt-2">Consulting commercial embroidery digitizing advisor…</p>
                </div>
              )}

              {aiAnswer && !askingAi && (
                <div
                  className="ai-answer-card"
                  style={{
                    background: "var(--card-bg, #f8fafc)",
                    border: "1px solid var(--border-color, #e2e8f0)",
                    borderRadius: "8px",
                    padding: "16px",
                    lineHeight: "1.6",
                    fontSize: "13px",
                    whiteSpace: "pre-wrap",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: "6px", marginBottom: "8px", color: "var(--accent)" }}>
                    <Bot size={18} />
                    <b>Digitizing & Production Advice:</b>
                  </div>
                  {aiAnswer}
                </div>
              )}
            </div>
          )}
        </div>

        <div className="modal-footer">
          {activeTab === "catalog" ? (
            !suggestion && !analyzing && !appliedNotice ? (
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
            ) : suggestion && !appliedNotice ? (
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
            ) : null
          ) : (
            <button className="secondary" onClick={onClose}>
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
