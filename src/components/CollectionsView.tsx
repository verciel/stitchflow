import React, { useState } from "react";
import { Folder, FolderPlus, MoreVertical, Plus, Trash2 } from "lucide-react";
import { createCollection, deleteCollection } from "../lib";
import type { Collection } from "../types";

interface CollectionsViewProps {
  collections: Collection[];
  onSelectCollection: (col: Collection) => void;
  onRefresh: () => void;
}

export const CollectionsView: React.FC<CollectionsViewProps> = ({
  collections,
  onSelectCollection,
  onRefresh,
}) => {
  const [showModal, setShowModal] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    try {
      setSubmitting(true);
      await createCollection(name.trim(), description.trim());
      setName("");
      setDescription("");
      setShowModal(false);
      onRefresh();
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to create collection");
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (e: React.MouseEvent, id: string, title: string) => {
    e.stopPropagation();
    if (
      confirm(
        `Delete collection "${title}"? Designs inside this collection will NOT be deleted.`
      )
    ) {
      try {
        await deleteCollection(id);
        onRefresh();
      } catch (err) {
        console.error(err);
      }
    }
  };

  return (
    <div className="collections-container">
      <div className="mb-4">
        <p className="subtle">
          Group related embroidery designs into themed folders and seasonal series.
        </p>
      </div>

      <div className="collections-grid">

        {collections.map((col) => (
          <article
            key={col.id}
            className="collection-card"
            onClick={() => onSelectCollection(col)}
          >
            <div className="collection-icon">
              <Folder size={28} />
            </div>
            <div className="collection-details">
              <h3>{col.name}</h3>
              <p className="collection-desc">
                {col.description || "No description provided."}
              </p>
              <span className="collection-meta">
                {col.designCount} design{col.designCount === 1 ? "" : "s"}
              </span>
            </div>
            <button
              className="delete-collection-btn"
              onClick={(e) => handleDelete(e, col.id, col.name)}
              title="Delete collection"
            >
              <Trash2 size={16} />
            </button>
          </article>
        ))}

        <div
          className="create-collection-card"
          onClick={() => setShowModal(true)}
        >
          <Plus size={28} />
          <span>Create New Collection</span>
        </div>
      </div>

      {showModal && (
        <div className="modal-backdrop" onClick={() => setShowModal(false)}>
          <div className="modal compact-modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>New Collection</h2>
            </div>
            <form onSubmit={handleCreate}>
              <div className="modal-body">
                <div className="form-group">
                  <label>Collection Name</label>
                  <input
                    type="text"
                    required
                    placeholder="e.g. Summer Florals 2026"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    autoFocus
                  />
                </div>
                <div className="form-group">
                  <label>Description (Optional)</label>
                  <textarea
                    rows={3}
                    placeholder="Notes or themes for this collection…"
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                  />
                </div>
              </div>
              <div className="modal-footer">
                <button
                  type="button"
                  className="text-button"
                  onClick={() => setShowModal(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="primary" disabled={submitting}>
                  {submitting ? "Creating…" : "Create Collection"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
