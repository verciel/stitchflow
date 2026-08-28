import React, { useState } from "react";
import { Boxes, Edit3, Plus, Trash2 } from "lucide-react";
import { createJob, deleteJob, updateJob } from "../lib";
import type { Job } from "../types";

interface JobsViewProps {
  jobs: Job[];
  onSelectJob: (job: Job) => void;
  onRefresh: () => void;
}

export const JobsView: React.FC<JobsViewProps> = ({
  jobs,
  onSelectJob,
  onRefresh,
}) => {
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [showModal, setShowModal] = useState(false);
  const [title, setTitle] = useState("");
  const [notes, setNotes] = useState("");
  const [status, setStatus] = useState<Job["status"]>("draft");
  const [submitting, setSubmitting] = useState(false);

  const filteredJobs = jobs.filter((j) =>
    statusFilter === "all" ? true : j.status === statusFilter
  );

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    try {
      setSubmitting(true);
      await createJob(title.trim(), notes.trim(), status);
      setTitle("");
      setNotes("");
      setStatus("draft");
      setShowModal(false);
      onRefresh();
    } catch (err) {
      alert(err instanceof Error ? err.message : "Failed to create job");
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (e: React.MouseEvent, id: string, jobTitle: string) => {
    e.stopPropagation();
    if (confirm(`Delete job "${jobTitle}"? Linked designs will NOT be deleted.`)) {
      try {
        await deleteJob(id);
        onRefresh();
      } catch (err) {
        console.error(err);
      }
    }
  };

  return (
    <div className="jobs-view-container">
      <div className="mb-4">
        <div className="status-filter-tabs">
          {["all", "draft", "active", "completed", "archived"].map((st) => (
            <button
              key={st}
              className={`status-tab ${statusFilter === st ? "active" : ""}`}
              onClick={() => setStatusFilter(st)}
            >
              {st.toUpperCase()}
            </button>
          ))}
        </div>
      </div>


      <div className="jobs-list-grid">
        {filteredJobs.map((j) => (
          <article
            key={j.id}
            className="job-card-rich"
            onClick={() => onSelectJob(j)}
          >
            <div className="job-card-head">
              <span className={`job-status-pill ${j.status}`}>
                {j.status.toUpperCase()}
              </span>
              <button
                className="delete-collection-btn"
                onClick={(e) => handleDelete(e, j.id, j.title)}
                title="Delete job"
              >
                <Trash2 size={15} />
              </button>
            </div>
            <h3>{j.title}</h3>
            <p className="job-notes">{j.notes || "No production notes added."}</p>
            <div className="job-stats-footer">
              <span>{j.designCount} linked design(s)</span>
              <span>{j.artworkCount} artwork asset(s)</span>
            </div>
          </article>
        ))}

        <div
          className="create-collection-card"
          onClick={() => setShowModal(true)}
        >
          <Plus size={28} />
          <span>Create New Job Container</span>
        </div>
      </div>


      {showModal && (
        <div className="modal-backdrop" onClick={() => setShowModal(false)}>
          <div className="modal compact-modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Create Job Container</h2>
            </div>
            <form onSubmit={handleCreate}>
              <div className="modal-body">
                <div className="form-group">
                  <label>Job Title</label>
                  <input
                    type="text"
                    required
                    placeholder="e.g. Polo Crest Order #4082"
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                    autoFocus
                  />
                </div>
                <div className="form-group">
                  <label>Status</label>
                  <select
                    value={status}
                    onChange={(e) => setStatus(e.target.value as Job["status"])}
                  >
                    <option value="draft">Draft (Planning)</option>
                    <option value="active">Active (In Production)</option>
                    <option value="completed">Completed</option>
                    <option value="archived">Archived</option>
                  </select>
                </div>
                <div className="form-group">
                  <label>Notes (Technical & Placement Info)</label>
                  <textarea
                    rows={3}
                    placeholder="Thread colors, backing, fabric type, machine notes…"
                    value={notes}
                    onChange={(e) => setNotes(e.target.value)}
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
                  {submitting ? "Creating…" : "Save Job"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
