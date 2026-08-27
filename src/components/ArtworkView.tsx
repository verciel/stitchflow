import React, { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FileText,
  FolderOpen,
  Image as ImageIcon,
  Plus,
  RefreshCw,
  Trash2,
} from "lucide-react";
import {
  deleteArtwork,
  importFiles,
  listArtwork,
  readImageData,
  revealInFolder,
} from "../lib";
import type { ArtworkAsset } from "../types";

export const ArtworkView: React.FC = () => {
  const [assets, setAssets] = useState<ArtworkAsset[]>([]);
  const [loading, setLoading] = useState(true);
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});

  const reload = async () => {
    try {
      setLoading(true);
      const list = await listArtwork();
      setAssets(list);

      // Load image data for image assets
      for (const item of list) {
        if (
          item.mimeType.includes("image") &&
          !item.mimeType.includes("pdf") &&
          !thumbnails[item.id]
        ) {
          readImageData(item.managedPath)
            .then((data) => {
              setThumbnails((prev) => ({ ...prev, [item.id]: data }));
            })
            .catch(() => {});
        }
      }
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reload();
  }, []);

  const handleImport = async () => {
    try {
      const selection = await open({
        multiple: true,
        filters: [
          {
            name: "Artwork Assets",
            extensions: ["png", "jpg", "jpeg", "svg", "pdf"],
          },
        ],
      });

      if (!selection) return;
      const paths = Array.isArray(selection) ? selection : [selection];
      await importFiles(paths, "skip");
      await reload();
    } catch (err) {
      console.error(err);
    }
  };

  const handleDelete = async (id: string, name: string) => {
    if (confirm(`Remove artwork asset "${name}"?`)) {
      try {
        await deleteArtwork(id);
        await reload();
      } catch (err) {
        console.error(err);
      }
    }
  };

  return (
    <div className="artwork-view-container">
      <div className="view-toolbar">
        <div>
          <h2>Source Artwork</h2>
          <p className="subtle">
            Manage vector logos, sketches, and customer mockups (PNG, JPG, SVG, PDF) linked to designs and jobs.
          </p>
        </div>
        <button className="primary" onClick={handleImport}>
          <Plus size={16} /> Import Artwork
        </button>
      </div>

      {loading && assets.length === 0 ? (
        <div className="empty-box">
          <RefreshCw size={24} className="spin text-accent" />
          <p>Loading artwork assets…</p>
        </div>
      ) : assets.length === 0 ? (
        <div className="empty-box">
          <ImageIcon size={36} />
          <h3>No artwork assets imported yet</h3>
          <p>Import PNG, JPG, SVG, or PDF artwork files to link with embroidery designs.</p>
          <button className="primary mt-3" onClick={handleImport}>
            Import Artwork
          </button>
        </div>
      ) : (
        <div className="artwork-grid">
          {assets.map((asset) => (
            <article key={asset.id} className="artwork-card">
              <div className="artwork-preview">
                {thumbnails[asset.id] ? (
                  <img
                    src={thumbnails[asset.id]}
                    alt={asset.filename}
                    className="artwork-img"
                  />
                ) : (
                  <div className="artwork-placeholder">
                    {asset.mimeType.includes("pdf") ? (
                      <FileText size={36} className="text-red-400" />
                    ) : (
                      <ImageIcon size={36} className="text-blue-400" />
                    )}
                    <span>{asset.filename.split(".").pop()?.toUpperCase()}</span>
                  </div>
                )}
              </div>
              <div className="artwork-info">
                <h4 className="truncate" title={asset.filename}>
                  {asset.filename}
                </h4>
                <div className="artwork-meta-row">
                  <span>{(asset.sizeBytes / 1024).toFixed(1)} KB</span>
                  <span>{new Date(asset.importedAt).toLocaleDateString()}</span>
                </div>
                <div className="artwork-actions">
                  <button
                    className="secondary icon-btn-sm"
                    onClick={() => revealInFolder(asset.managedPath)}
                    title="Reveal in folder"
                  >
                    <FolderOpen size={14} />
                  </button>
                  <button
                    className="secondary icon-btn-sm text-red"
                    onClick={() => handleDelete(asset.id, asset.filename)}
                    title="Delete artwork"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
};
