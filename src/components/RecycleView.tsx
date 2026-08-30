import React from "react";
import { ArchiveRestore, Trash2, Undo2 } from "lucide-react";
import { confirmDialog, emptyRecycleBin, permanentDeleteDesign, restoreDesign } from "../lib";
import type { Design } from "../types";
import { DesignImage } from "./DesignImage";

interface RecycleViewProps {
  recycledDesigns: Design[];
  onSelectDesign: (design: Design) => void;
  onRefresh: () => void;
}

export const RecycleView: React.FC<RecycleViewProps> = ({
  recycledDesigns,
  onSelectDesign,
  onRefresh,
}) => {
  const handleRestore = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    try {
      await restoreDesign(id);
      onRefresh();
    } catch (err) {
      console.error(err);
    }
  };

  const handlePermanentDelete = async (
    e: React.MouseEvent,
    id: string,
    title: string
  ) => {
    e.stopPropagation();
    const ok = await confirmDialog(
      `Permanently delete "${title}"? This cannot be undone and deletes the file from disk.`,
      "Delete Permanently"
    );
    if (ok) {
      try {
        await permanentDeleteDesign(id);
        onRefresh();
      } catch (err) {
        console.error(err);
      }
    }
  };

  const handleEmptyAll = async () => {
    const ok = await confirmDialog(
      `Permanently delete all ${recycledDesigns.length} items in the recycle area? This cannot be recovered.`,
      "Empty Recycle Bin"
    );
    if (ok) {
      try {
        await emptyRecycleBin();
        onRefresh();
      } catch (err) {
        console.error(err);
      }
    }
  };

  return (
    <div className="recycle-view-container">
      <div className="flex justify-between items-center mb-4">
        <p className="subtle">
          Deleted designs are quarantined here safely before permanent removal.
        </p>
        {recycledDesigns.length > 0 && (
          <button className="danger" onClick={handleEmptyAll}>
            <Trash2 size={16} /> Empty Recycle Bin
          </button>
        )}
      </div>

      {recycledDesigns.length === 0 ? (

        <div className="empty-box">
          <ArchiveRestore size={36} />
          <h3>Recycle area is empty</h3>
          <p>When you delete designs from your library, they will appear here safely before permanent removal.</p>
        </div>
      ) : (
        <div className="design-grid">
          {recycledDesigns.map((d) => (
            <article
              key={d.id}
              className="design-card recycled"
              onClick={() => onSelectDesign(d)}
            >
              <DesignImage
                previewPath={d.previewPath}
                title={d.title}
                format={d.format}
              />
              <div className="card-info">
                <div>
                  <h3>{d.title}</h3>
                  <p className="text-xs text-subtle">{d.filename}</p>
                </div>
                <span className="format-badge">{d.format}</span>
              </div>
              <div className="recycled-card-actions">
                <button
                  className="secondary compact-btn flex-1"
                  onClick={(e) => handleRestore(e, d.id)}
                >
                  <Undo2 size={14} /> Restore
                </button>
                <button
                  className="danger compact-btn"
                  onClick={(e) => handlePermanentDelete(e, d.id, d.title)}
                  title="Delete permanently"
                >
                  <Trash2 size={14} />
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
};
