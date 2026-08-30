import React, { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Archive,
  CheckCircle2,
  Database,
  ExternalLink,
  FolderOpen,
  HelpCircle,
  KeyRound,
  Lock,
  RefreshCw,
  Save,
  Server,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import {
  createBackup,
  formatError,
  getAiConfig,
  getInkstitchConfig,
  getSettings,
  restoreBackup,
  saveAiConfig,
  saveSetting,
  setInkstitchConfig,

  testAiConnection,
  testHfConnection,
  validateBackup,
} from "../lib";

import type { AiConfig, InkstitchConfig } from "../types";

interface SettingsViewProps {
  onRefresh?: () => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({ onRefresh }) => {
  // AI Settings State
  const [aiConfig, setAiConfigState] = useState<AiConfig>({
    endpoint: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    apiKey: "",
    enabled: false,
  });
  const [testingAi, setTestingAi] = useState(false);
  const [aiTestResult, setAiTestResult] = useState<{ success: boolean; msg: string } | null>(null);
  const [savingAi, setSavingAi] = useState(false);

  // Hugging Face Generator State
  const [hfToken, setHfToken] = useState("");
  const [hfModel, setHfModel] = useState("");
  const [testingHf, setTestingHf] = useState(false);
  const [hfTestResult, setHfTestResult] = useState<{ success: boolean; msg: string } | null>(null);




  // Ink/Stitch State
  const [inkstitch, setInkstitchState] = useState<InkstitchConfig>({
    inkscapePath: "",
    isConfigured: false,
  });

  // Backup & Restore State
  const [backupMsg, setBackupMsg] = useState<string | null>(null);
  const [isBackingUp, setIsBackingUp] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);

  useEffect(() => {
    void Promise.all([getAiConfig(), getInkstitchConfig(), getSettings()]).then(([ai, ink, settings]) => {
      setAiConfigState(ai);
      setInkstitchState(ink);
      if (settings.hf_api_token) setHfToken(settings.hf_api_token);
      if (settings.hf_model) setHfModel(settings.hf_model);
    });
  }, []);

  const handleSaveAi = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      setSavingAi(true);
      const toSave = {
        ...aiConfig,
        enabled:
          aiConfig.enabled ||
          Boolean(aiConfig.apiKey.trim() || !aiConfig.endpoint.includes("api.openai.com")),
      };
      await saveAiConfig(toSave);
      await saveSetting("hf_api_token", hfToken);
      await saveSetting("hf_model", hfModel);
      setAiConfigState(toSave);
      onRefresh?.();
      alert("AI & Hugging Face settings saved successfully.");
    } catch (err) {
      alert(formatError(err, "Failed to save AI config"));
    } finally {
      setSavingAi(false);
    }
  };

  const handleTestAi = async () => {

    try {
      setTestingAi(true);
      setAiTestResult(null);
      const res = await testAiConnection(aiConfig);
      setAiTestResult({ success: true, msg: res });
      // Auto-save and enable on successful connection test
      if (aiConfig.apiKey.trim() || !aiConfig.endpoint.includes("api.openai.com")) {
        const toSave = { ...aiConfig, enabled: true };
        await saveAiConfig(toSave);
        setAiConfigState(toSave);
        onRefresh?.();
      }
    } catch (err) {
      setAiTestResult({
        success: false,
        msg: formatError(err, "Connection failed"),
      });
    } finally {
      setTestingAi(false);
    }
  };

  const handleTestHf = async () => {
    try {
      setTestingHf(true);
      setHfTestResult(null);
      const res = await testHfConnection(hfToken, hfModel);
      setHfTestResult({ success: true, msg: res });
      await saveSetting("hf_api_token", hfToken);
      await saveSetting("hf_model", hfModel);
    } catch (err) {
      setHfTestResult({
        success: false,
        msg: formatError(err, "Hugging Face connection failed"),
      });
    } finally {
      setTestingHf(false);
    }
  };

  const handleBrowseInkscape = async () => {


    try {
      const file = await open({
        multiple: false,
        filters: [{ name: "Inkscape Executable", extensions: ["exe"] }],
      });
      if (file && typeof file === "string") {
        await setInkstitchConfig(file);
        setInkstitchState({ inkscapePath: file, isConfigured: true });
      }
    } catch (err) {
      console.error(err);
    }
  };

  const handleCreateBackup = async () => {
    try {
      setIsBackingUp(true);
      setBackupMsg(null);
      const folder = await open({
        directory: true,
        multiple: false,
        title: "Select Destination Folder for Backup Archive",
      });

      const dest = folder && typeof folder === "string" ? folder : undefined;
      const backupPath = await createBackup(dest);
      setBackupMsg(`Backup archive created successfully: ${backupPath}`);
    } catch (err) {
      setBackupMsg(err instanceof Error ? err.message : "Backup failed");
    } finally {
      setIsBackingUp(false);
    }
  };

  const handleRestoreBackup = async () => {
    try {
      setIsRestoring(true);
      setBackupMsg(null);

      // 1. Pick ZIP archive
      const archive = await open({
        multiple: false,
        filters: [{ name: "Stitchflow Backup Archive (*.zip)", extensions: ["zip"] }],
      });

      if (!archive || typeof archive !== "string") {
        setIsRestoring(false);
        return;
      }

      // 2. Validate
      const manifest = await validateBackup(archive);

      // 3. Pick restore directory (must not overwrite active library)
      const targetDir = await open({
        directory: true,
        multiple: false,
        title: `Select folder to unpack validated backup (${manifest.designCount} designs)`,
      });

      if (!targetDir || typeof targetDir !== "string") {
        setIsRestoring(false);
        return;
      }

      const resPath = await restoreBackup(archive, targetDir);
      setBackupMsg(`Archive successfully validated and restored to: ${resPath}`);
    } catch (err) {
      setBackupMsg(err instanceof Error ? err.message : "Restore validation failed");
    } finally {
      setIsRestoring(false);
    }
  };

  return (
    <div className="settings-view-container">
      <div className="settings-sections-stack">
        {/* AI & Vision Provider Configuration */}

        <section className="settings-card">
          <div className="settings-card-header">
            <div className="settings-icon-title">
              <Sparkles size={20} className="text-accent" />
              <div>
                <h3>AI Vision Analysis Provider</h3>
                <p className="subtle text-xs">
                  Connect to an OpenAI-compatible vision endpoint (OpenAI, Ollama, LM Studio, or OpenRouter).
                </p>
              </div>
            </div>
          </div>

          <div className="privacy-notice-box">
            <ShieldCheck size={18} className="text-green-500" />
            <div>
              <b>Zero-Originals Privacy Guarantee:</b>
              <p>
                Stitchflow transmits ONLY rendered 2D preview PNG images and extracted technical metadata. Your original embroidery files are NEVER uploaded. All suggestions require your manual approval before catalog changes occur.
              </p>
            </div>
          </div>

          <form onSubmit={handleSaveAi} className="settings-form">
            <div className="form-toggle-row">
              <div>
                <b>Enable AI Features</b>
                <p className="subtle text-xs">
                  Turn on AI catalog tagging, visual descriptions, and workflow assessments.
                </p>
              </div>
              <label className="toggle-switch">
                <input
                  type="checkbox"
                  checked={aiConfig.enabled}
                  onChange={(e) =>
                    setAiConfigState({ ...aiConfig, enabled: e.target.checked })
                  }
                />
                <span className="slider" />
              </label>
            </div>

            <div className="form-group">
              <label>API Endpoint Base URL</label>
              <div className="input-with-icon">
                <Server size={16} />
                <input
                  type="text"
                  placeholder="https://api.openai.com/v1"
                  value={aiConfig.endpoint}
                  onChange={(e) =>
                    setAiConfigState({ ...aiConfig, endpoint: e.target.value })
                  }
                />
              </div>
              <small className="help-text">
                Compatible with official OpenAI, Local LM Studio (http://localhost:1234/v1), or Ollama (http://localhost:11434/v1).
              </small>
            </div>

            <div className="form-group">
              <label>Model Identifier</label>
              <input
                type="text"
                placeholder="gpt-4o-mini, gpt-4o, or llava"
                value={aiConfig.model}
                onChange={(e) =>
                  setAiConfigState({ ...aiConfig, model: e.target.value })
                }
              />
            </div>

            <div className="form-group">
              <label>API Key (Saved locally)</label>
              <div className="input-with-icon">
                <KeyRound size={16} />
                <input
                  type="password"
                  placeholder="sk-… (OpenAI / Groq API key)"
                  value={aiConfig.apiKey}
                  onChange={(e) =>
                    setAiConfigState({ ...aiConfig, apiKey: e.target.value })
                  }
                />
              </div>
              <small className="help-text">Leave blank for local unauthenticated models.</small>
            </div>

            <div style={{ marginBottom: "16px" }}>
              <button
                type="button"
                className="secondary compact-btn"
                onClick={handleTestAi}
                disabled={testingAi || !aiConfig.endpoint}
              >
                {testingAi ? <RefreshCw size={14} className="spin" /> : <Server size={14} />}
                <span>Test Catalog AI Connection</span>
              </button>

              {aiTestResult && (
                <div
                  className={`alert-banner mt-2 ${aiTestResult.success ? "alert-success" : "alert-error"}`}
                >
                  {aiTestResult.success ? <CheckCircle2 size={16} /> : <ShieldCheck size={16} />}
                  <span>{aiTestResult.msg}</span>
                </div>
              )}
            </div>

            <hr style={{ borderColor: "var(--border-color)", margin: "20px 0" }} />

            <div style={{ marginBottom: "12px" }}>
              <b className="text-sm" style={{ color: "var(--accent)" }}>Design Creation Engine</b>
              <p className="subtle text-xs mt-1">
                Generate new companion designs and save them directly to your library. Works out-of-the-box with zero configuration.
              </p>
            </div>

            <div className="form-group">
              <label>Custom Endpoint URL (Optional)</label>
              <input
                type="text"
                value={hfModel}
                onChange={(e) => setHfModel(e.target.value)}
                placeholder="Leave blank for built-in generator, or paste custom endpoint URL"
                style={{ width: "100%", padding: "8px 10px" }}
              />
              <small className="help-text">Leave blank to use the built-in generator, or paste your custom Hugging Face / private cloud URL.</small>
            </div>

            <div className="form-group">
              <label>API Access Token (Optional)</label>
              <div className="input-with-icon">
                <KeyRound size={16} />
                <input
                  type="password"
                  placeholder="hf_… (Only required for authenticated private endpoints)"
                  value={hfToken}
                  onChange={(e) => setHfToken(e.target.value)}
                />
              </div>
              <small className="help-text">Optional: enter your token only if using a private custom endpoint.</small>
            </div>

            <div style={{ marginBottom: "20px" }}>
              <button
                type="button"
                className="secondary compact-btn"
                onClick={handleTestHf}
                disabled={testingHf}
              >
                {testingHf ? <RefreshCw size={14} className="spin" /> : <Sparkles size={14} />}
                <span>Test Design Generator</span>
              </button>

              {hfTestResult && (
                <div
                  className={`alert-banner mt-2 ${hfTestResult.success ? "alert-success" : "alert-error"}`}
                >
                  {hfTestResult.success ? <CheckCircle2 size={16} /> : <ShieldCheck size={16} />}
                  <span>{hfTestResult.msg}</span>
                </div>
              )}
            </div>


            <div className="form-actions-split" style={{ borderTop: "1px solid var(--border-color)", paddingTop: "16px" }}>
              <div />
              <button type="submit" className="primary" disabled={savingAi}>
                <Save size={15} />
                <span>Save All AI & Hugging Face Settings</span>
              </button>
            </div>

          </form>
        </section>

        {/* Ink/Stitch Handoff Configuration */}
        <section className="settings-card">
          <div className="settings-card-header">
            <div className="settings-icon-title">
              <ExternalLink size={20} />
              <div>
                <h3>Ink/Stitch & Inkscape Integration</h3>
                <p className="subtle text-xs">
                  Launch designs directly into Inkscape with the Ink/Stitch extension installed.
                </p>
              </div>
            </div>
          </div>

          <div className="settings-form">
            <div className="form-group">
              <label>Inkscape Executable Path</label>
              <div className="input-action-row">
                <input
                  type="text"
                  placeholder="C:\Program Files\Inkscape\bin\inkscape.exe"
                  value={inkstitch.inkscapePath}
                  readOnly
                />
                <button
                  type="button"
                  className="secondary"
                  onClick={handleBrowseInkscape}
                >
                  <FolderOpen size={16} /> Browse…
                </button>
              </div>
              <span
                className={`config-badge ${inkstitch.isConfigured ? "configured" : "unconfigured"}`}
              >
                {inkstitch.isConfigured ? "Executable Ready" : "Path Not Configured"}
              </span>
            </div>
          </div>
        </section>

        {/* Backup and Restore */}
        <section className="settings-card">
          <div className="settings-card-header">
            <div className="settings-icon-title">
              <Archive size={20} />
              <div>
                <h3>Portable Backups & Restoration</h3>
                <p className="subtle text-xs">
                  Create single-file ZIP archives containing SQLite data, checksums, and managed files.
                </p>
              </div>
            </div>
          </div>

          <div className="backup-controls-grid">
            <div className="backup-box">
              <b>Create Library Backup</b>
              <p className="subtle text-xs mb-3">
                Bundles all active designs, artwork, stitch previews, and database into a verifiable ZIP archive.
              </p>
              <button
                className="secondary"
                onClick={handleCreateBackup}
                disabled={isBackingUp}
              >
                {isBackingUp ? <RefreshCw size={15} className="spin" /> : <Database size={15} />}
                <span>Create Backup (.zip)</span>
              </button>
            </div>

            <div className="backup-box">
              <b>Restore from Backup</b>
              <p className="subtle text-xs mb-3">
                Validates checksum integrity from manifest.json and restores into an isolated library directory.
              </p>
              <button
                className="secondary"
                onClick={handleRestoreBackup}
                disabled={isRestoring}
              >
                {isRestoring ? <RefreshCw size={15} className="spin" /> : <Archive size={15} />}
                <span>Restore Backup…</span>
              </button>
            </div>
          </div>

          {backupMsg && (
            <div className="alert-banner alert-success mt-3">
              <CheckCircle2 size={16} />
              <span>{backupMsg}</span>
            </div>
          )}
        </section>
      </div>
    </div>
  );
};
