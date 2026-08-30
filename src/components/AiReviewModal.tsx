import React, { useEffect, useState } from "react";
import {
  AlertCircle,
  ArrowRight,
  Bot,
  Check,
  CheckCircle2,
  Cpu,
  Download,
  HelpCircle,
  Image as ImageIcon,
  Layers,
  Palette,
  RefreshCw,
  Send,
  ShieldCheck,
  Sparkles,
  Tag as TagIcon,
  Wand2,
  X,
} from "lucide-react";

import {
  analyzeDesigns,
  applyAiSuggestion,
  digitizeAndImportDesign,
  formatError,
  generateAiDesignImage,
  getAiConfig,
  saveAiConfig,
} from "../lib";
import type {
  AiConfig,
  AiSuggestion,
  Design,
  GeneratedArtworkResult,
} from "../types";
import { DesignImage } from "./DesignImage";


interface AiReviewModalProps {
  design: Design | null;
  aiConfig: AiConfig;
  isOpen: boolean;
  onClose: () => void;
  onApplied: () => void;
  onSelectDesign?: (design: Design) => void;
  onOpenSettings?: () => void;
}


export const AiReviewModal: React.FC<AiReviewModalProps> = ({
  design,
  aiConfig: initialConfig,
  isOpen,
  onClose,
  onApplied,
  onSelectDesign,
  onOpenSettings,
}) => {
  const [activeTab, setActiveTab] = useState<"catalog" | "generator">("catalog");
  const [currentConfig, setCurrentConfig] = useState<AiConfig>(initialConfig);
  const [analyzing, setAnalyzing] = useState(false);
  const [suggestion, setSuggestion] = useState<AiSuggestion | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appliedNotice, setAppliedNotice] = useState(false);

  // AI Generator & Auto-Digitizer State
  const [genPrompt, setGenPrompt] = useState("");
  const [selectedStyle, setSelectedStyle] = useState<string>("patch");
  const [generatingImg, setGeneratingImg] = useState(false);
  const [generatedResult, setGeneratedResult] = useState<GeneratedArtworkResult | null>(null);
  
  // Digitizing Options
  const [digitizeTitle, setDigitizeTitle] = useState("");
  const [digitizeFormat, setDigitizeFormat] = useState("PES");
  const [digitizeWidth, setDigitizeWidth] = useState(50);
  const [digitizeHeight, setDigitizeHeight] = useState(50);
  const [digitizing, setDigitizing] = useState(false);
  const [digitizedDesign, setDigitizedDesign] = useState<Design | null>(null);

  // Always refresh latest AI config from database when modal opens
  useEffect(() => {
    if (isOpen && design) {
      setError(null);
      setSuggestion(null);
      setAppliedNotice(false);
      setGeneratedResult(null);
      setDigitizedDesign(null);
      setGenPrompt("");
      setSelectedStyle("patch");
      setDigitizeTitle(`${design.title} Companion`);
      setDigitizeFormat(design.format || "PES");
      setDigitizeWidth(Math.round(design.widthMm || 50));
      setDigitizeHeight(Math.round(design.heightMm || 50));

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
  }, [isOpen, design]);


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
    if (!design) return;
    try {
      setAnalyzing(true);
      setError(null);
      setAppliedNotice(false);

      if (!currentConfig.enabled) {
        await handleEnableAi();
      }

      const results = await analyzeDesigns([design.id]);
      if (results.length > 0) {
        setSuggestion(results[0]);
      } else {
        setError("Analysis complete, but no new tags or metadata were identified for this design.");
      }
    } catch (err) {
      setError(formatError(err, "Analysis failed"));
    } finally {
      setAnalyzing(false);
    }
  };

  const handleApply = async (apply: boolean) => {
    if (!suggestion || !design) return;
    try {
      setError(null);
      await applyAiSuggestion(suggestion.id, apply);
      if (apply) {
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

  // Generate Visual Artwork via Diffusion Engine with Selected Style
  const handleGenerateArtwork = async (promptOverride?: string) => {
    const p = (promptOverride || genPrompt).trim();
    try {
      setGeneratingImg(true);
      setError(null);
      setDigitizedDesign(null);
      if (promptOverride) setGenPrompt(promptOverride);

      const res = await generateAiDesignImage(design.id, p || undefined, selectedStyle);
      setGeneratedResult(res);
    } catch (err) {
      setError(formatError(err, "Artwork generation failed"));
    } finally {
      setGeneratingImg(false);
    }
  };

  // Auto-Digitize Generated PNG into Machine Embroidery File
  const handleDigitizeAndSave = async () => {
    if (!generatedResult) return;
    try {
      setDigitizing(true);
      setError(null);

      const newDesign = await digitizeAndImportDesign({
        sourceImagePath: generatedResult.tempPath,
        title: digitizeTitle.trim() || `${design.title}_AI_Digitized`,
        targetFormat: digitizeFormat,
        widthMm: digitizeWidth,
        heightMm: digitizeHeight,
        tags: ["ai-generated", "auto-digitized", design.format.toLowerCase()],
        category: design.aiCategory || "AI Generated",
      });

      setDigitizedDesign(newDesign);
      onApplied();
    } catch (err) {
      setError(formatError(err, "Auto-digitizing failed"));
    } finally {
      setDigitizing(false);
    }
  };

  const isOpenAiMissingKey =
    currentConfig.endpoint.includes("api.openai.com") && !currentConfig.apiKey.trim();

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal ai-review-modal" style={{ maxWidth: "800px" }} onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title-group">
            <Sparkles size={22} className="text-accent" />
            <div>
              <h2>AI Embroidery Studio</h2>
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
            ✨ Catalog Tagging
          </button>
          <button
            type="button"
            className={`tab-btn ${activeTab === "generator" ? "active font-bold text-accent" : "subtle"}`}
            style={{ padding: "8px 16px", borderBottom: activeTab === "generator" ? "2px solid var(--accent)" : "none", background: "none", cursor: "pointer" }}
            onClick={() => setActiveTab("generator")}
          >
            🎨 Create New Design
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
                <h3>Analyzing stitch pattern…</h3>
                <p className="subtle text-xs">
                  Generating catalog classifications, proposed tags, and thread color insights.
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
            /* AI Design Generator Tab */
            <div className="ai-generator-panel">
              <div className="generator-intro-card" style={{ background: "var(--card-bg, #f8fafc)", border: "1px solid var(--border-color, #e2e8f0)", borderRadius: "8px", padding: "14px 16px", marginBottom: "16px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "8px", color: "var(--accent)", marginBottom: "4px" }}>
                  <Wand2 size={18} />
                  <b>Create Matching Design</b>
                </div>
                <p className="subtle text-xs" style={{ margin: 0 }}>
                  Create new matching designs for <b>{design.title}</b> and save them directly to your library.
                </p>
              </div>

              {/* Custom Prompt Input */}
              <div className="gen-input-row" style={{ display: "flex", gap: "8px", marginBottom: "14px" }}>
                <input
                  type="text"
                  placeholder="Describe what to create (e.g. 'witch flying on broom', 'garden sunflower', 'flying eagle')…"
                  value={genPrompt}
                  onChange={(e) => setGenPrompt(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && genPrompt.trim() && !generatingImg) handleGenerateArtwork();
                  }}
                  disabled={generatingImg || digitizing}
                  style={{ flex: 1 }}
                />
                <button
                  type="button"
                  className="primary"
                  onClick={() => handleGenerateArtwork()}
                  disabled={!genPrompt.trim() || generatingImg || digitizing}
                >
                  {generatingImg ? <RefreshCw size={16} className="spin" /> : <Wand2 size={16} />}
                  <span>Create Design</span>
                </button>
              </div>

              {/* Embroidery Style Mode Selector */}
              <div style={{ marginBottom: "16px" }}>
                <label className="text-xs font-bold text-subtle" style={{ display: "block", marginBottom: "6px" }}>
                  EMBROIDERY STYLE PRESET
                </label>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "6px" }}>
                  {[
                    { id: "patch", label: "🎨 3-Color Patch", desc: "Bold black borders & flat solid fills" },
                    { id: "silhouette", label: "🖤 Silhouette", desc: "Solid black cutout, zero gradients" },
                    { id: "line_art", label: "🖋️ Line Art", desc: "Single continuous running stitch" },
                    { id: "crest", label: "🛡️ Varsity Crest", desc: "Collegiate shield & laurel emblem" },
                    { id: "floral", label: "🌸 Folk Floral", desc: "Stylized botanical petals" },
                    { id: "applique", label: "🧸 Appliqué", desc: "Large simplified shapes" },
                  ].map((st) => (
                    <button
                      key={st.id}
                      type="button"
                      className={`compact-btn ${selectedStyle === st.id ? "primary" : "secondary"}`}
                      style={{
                        textAlign: "left",
                        padding: "6px 10px",
                        display: "flex",
                        flexDirection: "column",
                        border: selectedStyle === st.id ? "2px solid var(--accent)" : "1px solid var(--border-color)",
                      }}
                      onClick={() => setSelectedStyle(st.id)}
                      disabled={generatingImg || digitizing}
                    >
                      <span style={{ fontWeight: "bold", fontSize: "12px" }}>{st.label}</span>
                      <span style={{ fontSize: "10px", opacity: 0.8 }}>{st.desc}</span>
                    </button>
                  ))}
                </div>
              </div>



              {generatingImg && (
                <div className="ai-analyzing-box" style={{ padding: "30px" }}>
                  <RefreshCw size={32} className="spin text-accent" />
                  <h4 style={{ margin: "10px 0 4px" }}>Creating your design…</h4>
                  <p className="subtle text-xs">Designing embroidery motif and preparing stitches.</p>
                </div>
              )}

              {/* Generated Artwork & Digitizing Controls */}
              {generatedResult && !generatingImg && (
                <div
                  className="generated-artwork-card"
                  style={{
                    background: "var(--card-bg, #ffffff)",
                    border: "1px solid var(--border-color, #e2e8f0)",
                    borderRadius: "8px",
                    padding: "16px",
                    marginTop: "16px",
                  }}
                >
                  <div style={{ display: "grid", gridTemplateColumns: "180px 1fr", gap: "16px" }}>
                    {/* Artwork Preview Image */}
                    <div style={{ textAlign: "center" }}>
                      <div
                        style={{
                          width: "180px",
                          height: "180px",
                          background: "#ffffff",
                          border: "1px solid var(--border-color, #cbd5e1)",
                          borderRadius: "8px",
                          overflow: "hidden",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                        }}
                      >
                        <img
                          src={generatedResult.imageData}
                          alt="Generated Design Preview"
                          style={{ maxWidth: "100%", maxHeight: "100%", objectFit: "contain" }}
                        />
                      </div>
                      <span className="subtle text-xs mt-2" style={{ display: "block" }}>
                        Design Preview
                      </span>
                    </div>

                    {/* Auto-Digitizing Configuration Form */}
                    <div className="digitize-controls" style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                      <div>
                        <label className="text-xs font-bold text-subtle" style={{ display: "block", marginBottom: "4px" }}>
                          DESIGN TITLE
                        </label>
                        <input
                          type="text"
                          value={digitizeTitle}
                          onChange={(e) => setDigitizeTitle(e.target.value)}
                          disabled={digitizing}
                          style={{ width: "100%", padding: "6px 10px", fontSize: "13px" }}
                        />
                      </div>

                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "8px" }}>
                        <div>
                          <label className="text-xs font-bold text-subtle" style={{ display: "block", marginBottom: "4px" }}>
                            FORMAT
                          </label>
                          <select
                            value={digitizeFormat}
                            onChange={(e) => setDigitizeFormat(e.target.value)}
                            disabled={digitizing}
                            style={{ width: "100%", padding: "6px 8px", fontSize: "13px" }}
                          >
                            <option value="PES">PES (Brother)</option>
                            <option value="DST">DST (Tajima)</option>
                            <option value="JEF">JEF (Janome)</option>
                            <option value="VP3">VP3 (Pfaff/Husqvarna)</option>
                            <option value="EXP">EXP (Melco/Bernina)</option>
                            <option value="HUS">HUS (Husqvarna)</option>
                          </select>
                        </div>

                        <div>
                          <label className="text-xs font-bold text-subtle" style={{ display: "block", marginBottom: "4px" }}>
                            WIDTH (MM)
                          </label>
                          <input
                            type="number"
                            min="20"
                            max="300"
                            value={digitizeWidth}
                            onChange={(e) => setDigitizeWidth(Number(e.target.value))}
                            disabled={digitizing}
                            style={{ width: "100%", padding: "6px 8px", fontSize: "13px" }}
                          />
                        </div>

                        <div>
                          <label className="text-xs font-bold text-subtle" style={{ display: "block", marginBottom: "4px" }}>
                            HEIGHT (MM)
                          </label>
                          <input
                            type="number"
                            min="20"
                            max="300"
                            value={digitizeHeight}
                            onChange={(e) => setDigitizeHeight(Number(e.target.value))}
                            disabled={digitizing}
                            style={{ width: "100%", padding: "6px 8px", fontSize: "13px" }}
                          />
                        </div>
                      </div>

                      <div style={{ marginTop: "10px" }}>
                        <button
                          type="button"
                          className="primary"
                          onClick={handleDigitizeAndSave}
                          disabled={digitizing}
                          style={{ width: "100%", padding: "10px 16px", display: "flex", alignItems: "center", justifyContent: "center", gap: "8px" }}
                        >
                          {digitizing ? (
                            <>
                              <RefreshCw size={16} className="spin" />
                              <span>Calculating stitches & saving to library…</span>
                            </>
                          ) : (
                            <>
                              <Cpu size={16} />
                              <span>✨ Save Design to Library</span>
                            </>
                          )}
                        </button>
                      </div>

                      {digitizedDesign && (
                        <div
                          className="digitized-success-box"
                          style={{
                            background: "#f0fdf4",
                            border: "1px solid #bbf7d0",
                            borderRadius: "6px",
                            padding: "10px 14px",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "space-between",
                            marginTop: "8px",
                          }}
                        >
                          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                            <CheckCircle2 size={20} className="text-green-600" />
                            <div>
                              <b style={{ color: "#166534", fontSize: "13px" }}>Design Added to Library!</b>
                              <p style={{ margin: 0, fontSize: "11px", color: "#15803d" }}>
                                {digitizedDesign.stitches?.toLocaleString()} stitches · {digitizedDesign.widthMm?.toFixed(1)} × {digitizedDesign.heightMm?.toFixed(1)} mm ({digitizedDesign.format})
                              </p>
                            </div>
                          </div>
                          {onSelectDesign && (
                            <button
                              type="button"
                              className="secondary compact-btn"
                              onClick={() => {
                                onSelectDesign(digitizedDesign);
                                onClose();
                              }}
                            >
                              <span>View Design</span>
                              <ArrowRight size={14} />
                            </button>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
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
