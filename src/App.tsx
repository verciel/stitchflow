import React, { useEffect, useMemo, useState } from "react";
import {
  ArchiveRestore,
  Bot,
  Boxes,
  Clock,
  Filter,
  FolderOpen,
  Grid2X2,
  Image as ImageIcon,
  Import,
  LayoutList,
  Plus,
  RefreshCw,
  Search,
  Settings2,
  Sparkles,
  Tag as TagIcon,
  Trash2,
  X,
} from "lucide-react";
import {
  deleteDesign,
  getAiConfig,
  listCollections,
  listDesigns,
  listJobs,
  listTags,
  restoreDesign,
} from "./lib";
import type {
  AiConfig,
  Collection,
  Design,
  FilterOptions,
  Job,
  Tag,
} from "./types";
import { AiReviewModal } from "./components/AiReviewModal";
import { ArtworkView } from "./components/ArtworkView";
import { BatchImportModal } from "./components/BatchImportModal";
import { CollectionsView } from "./components/CollectionsView";
import { DesignDetailsDrawer } from "./components/DesignDetailsDrawer";
import { DesignImage } from "./components/DesignImage";
import { JobsView } from "./components/JobsView";
import { RecycleView } from "./components/RecycleView";
import { SettingsView } from "./components/SettingsView";

type Section = "library" | "collections" | "jobs" | "artwork" | "recycle" | "settings";

const FORMAT_OPTIONS = [
  "all",
  "DST",
  "PES",
  "JEF",
  "VP3",
  "EXP",
  "HUS",
  "XXX",
  "SEW",
  "PCS",
  "PEC",
];

export function App() {
  const [section, setSection] = useState<Section>("library");
  const [designs, setDesigns] = useState<Design[]>([]);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [recycledDesigns, setRecycledDesigns] = useState<Design[]>([]);

  // Search and Filter State
  const [query, setQuery] = useState("");
  const [selectedFormat, setSelectedFormat] = useState("all");
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [selectedCollectionId, setSelectedCollectionId] = useState<string | null>(null);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<FilterOptions["sortBy"]>("date_desc");

  // Selection & UI State
  const [selectedDesign, setSelectedDesign] = useState<Design | null>(null);
  const [isGridView, setIsGridView] = useState(true);
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);
  const [aiTargetDesign, setAiTargetDesign] = useState<Design | null>(null);
  const [aiConfig, setAiConfig] = useState<AiConfig>({
    endpoint: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    apiKey: "",
    enabled: false,
  });
  const [loading, setLoading] = useState(false);

  const reloadData = async () => {
    try {
      setLoading(true);
      const filterParams: FilterOptions = {
        query: query.trim() || undefined,
        format: selectedFormat !== "all" ? selectedFormat : undefined,
        tag: selectedTag || undefined,
        collectionId: selectedCollectionId || undefined,
        jobId: selectedJobId || undefined,
        status: "active",
        sortBy,
      };

      const [activeList, colList, jobList, tagList, aiConf, recList] =
        await Promise.all([
          listDesigns(filterParams),
          listCollections(),
          listJobs(),
          listTags(),
          getAiConfig(),
          listDesigns({ status: "recycled" }),
        ]);

      setDesigns(activeList);
      setCollections(colList);
      setJobs(jobList);
      setTags(tagList);
      setAiConfig(aiConf);
      setRecycledDesigns(recList);

      // Keep selected design fresh
      if (selectedDesign) {
        const fresh = activeList.find((d) => d.id === selectedDesign.id);
        if (fresh) setSelectedDesign(fresh);
      }
    } catch (err) {
      console.error("Failed to load catalog data:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reloadData();
  }, [query, selectedFormat, selectedTag, selectedCollectionId, selectedJobId, sortBy]);

  const handleResetFilters = () => {
    setQuery("");
    setSelectedFormat("all");
    setSelectedTag(null);
    setSelectedCollectionId(null);
    setSelectedJobId(null);
  };

  const hasActiveFilters =
    query ||
    selectedFormat !== "all" ||
    selectedTag ||
    selectedCollectionId ||
    selectedJobId;

  const activeCollectionName = useMemo(() => {
    if (!selectedCollectionId) return null;
    return collections.find((c) => c.id === selectedCollectionId)?.name;
  }, [selectedCollectionId, collections]);

  const activeJobName = useMemo(() => {
    if (!selectedJobId) return null;
    return jobs.find((j) => j.id === selectedJobId)?.title;
  }, [selectedJobId, jobs]);

  return (
    <main className="app-shell">
      {/* Primary Sidebar */}
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">S</div>
          <div className="brand-text">
            <span className="brand-title">Stitchflow</span>
            <span className="brand-subtitle">V1 Local Edition</span>
          </div>
        </div>

        <button
          className="import-button"
          onClick={() => setIsImportModalOpen(true)}
        >
          <Import size={18} />
          <span>Import designs</span>
        </button>

        <nav className="nav-group">
          <NavItem
            icon={<Grid2X2 size={18} />}
            label="Library"
            count={designs.length}
            active={section === "library"}
            onClick={() => {
              setSection("library");
              setSelectedCollectionId(null);
              setSelectedJobId(null);
            }}
          />
          <NavItem
            icon={<FolderOpen size={18} />}
            label="Collections"
            count={collections.length}
            active={section === "collections"}
            onClick={() => setSection("collections")}
          />
          <NavItem
            icon={<Boxes size={18} />}
            label="Jobs"
            count={jobs.length}
            active={section === "jobs"}
            onClick={() => setSection("jobs")}
          />
          <NavItem
            icon={<ImageIcon size={18} />}
            label="Artwork"
            active={section === "artwork"}
            onClick={() => setSection("artwork")}
          />
          <NavItem
            icon={<Trash2 size={18} />}
            label="Recycle area"
            count={recycledDesigns.length > 0 ? recycledDesigns.length : undefined}
            active={section === "recycle"}
            onClick={() => setSection("recycle")}
          />
        </nav>

        <div className="sidebar-bottom">
          <NavItem
            icon={<Settings2 size={18} />}
            label="Settings"
            active={section === "settings"}
            onClick={() => setSection("settings")}
          />
          <div className="account-card">
            <div className="account-avatar">L</div>
            <div className="account-info">
              <b>Local Library</b>
              <small>Offline First · SQLite FTS5</small>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Workspace Area */}
      <section className="workspace">
        {/* Top Header */}
        <header className="workspace-header">
          <div>
            <span className="eyebrow">
              {section === "library"
                ? activeCollectionName
                  ? `COLLECTION: ${activeCollectionName.toUpperCase()}`
                  : activeJobName
                  ? `JOB: ${activeJobName.toUpperCase()}`
                  : "MANAGED CATALOG"
                : section.toUpperCase()}
            </span>
            <h1>
              {section === "library"
                ? activeCollectionName || activeJobName || "Your Embroidery Designs"
                : section === "collections"
                ? "Collections"
                : section === "jobs"
                ? "Jobs"
                : section === "artwork"
                ? "Source Artwork Assets"
                : section === "recycle"
                ? "Recycle Area"
                : "Application Settings"}
            </h1>
            <p className="subtle">
              {section === "library"
                ? `${designs.length} designs indexed · Secure local storage`
                : "Organize, inspect, and export your embroidery work."}
            </p>
          </div>

          <div className="header-actions">
            {section === "library" && (
              <button
                className="secondary"
                onClick={() => {
                  if (selectedDesign) {
                    setAiTargetDesign(selectedDesign);
                  } else if (designs.length > 0) {
                    setAiTargetDesign(designs[0]);
                  }
                }}
                disabled={designs.length === 0}
                title="Run Vision AI on selected design"
              >
                <Sparkles size={16} className="text-accent" />
                <span>Analyze with AI</span>
              </button>
            )}

            <button
              className="primary"
              onClick={() => setIsImportModalOpen(true)}
            >
              <Plus size={17} />
              <span>Import files</span>
            </button>
          </div>
        </header>

        {/* Section View Routing */}
        {section === "collections" ? (
          <CollectionsView
            collections={collections}
            onSelectCollection={(col) => {
              setSelectedCollectionId(col.id);
              setSection("library");
            }}
            onRefresh={reloadData}
          />
        ) : section === "jobs" ? (
          <JobsView
            jobs={jobs}
            onSelectJob={(j) => {
              setSelectedJobId(j.id);
              setSection("library");
            }}
            onRefresh={reloadData}
          />
        ) : section === "artwork" ? (
          <ArtworkView />
        ) : section === "recycle" ? (
          <RecycleView
            recycledDesigns={recycledDesigns}
            onSelectDesign={(d) => setSelectedDesign(d)}
            onRefresh={reloadData}
          />
        ) : section === "settings" ? (
          <SettingsView />
        ) : (
          /* Main Library Section */
          <>
            {/* Filter and Search Bar */}
            <div className="toolbar">
              <div className="search-box">
                <Search size={18} className="search-icon" />
                <input
                  type="text"
                  placeholder="Search by title, filename, tags, or AI category…"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
                {query && (
                  <button
                    className="icon-button-sm"
                    onClick={() => setQuery("")}
                  >
                    <X size={14} />
                  </button>
                )}
              </div>

              {/* Tag Filter Dropdown */}
              <div className="filter-dropdown-wrapper">
                <select
                  value={selectedTag ?? ""}
                  onChange={(e) => setSelectedTag(e.target.value || null)}
                  className="filter-select"
                >
                  <option value="">All tags ({tags.length})</option>
                  {tags.map((t) => (
                    <option key={t.id} value={t.name}>
                      #{t.name} ({t.count})
                    </option>
                  ))}
                </select>
              </div>

              {/* Sort Dropdown */}
              <div className="filter-dropdown-wrapper">
                <select
                  value={sortBy}
                  onChange={(e) =>
                    setSortBy(e.target.value as FilterOptions["sortBy"])
                  }
                  className="filter-select"
                >
                  <option value="date_desc">Newest Imported</option>
                  <option value="date_asc">Oldest Imported</option>
                  <option value="stitches_desc">Most Stitches</option>
                  <option value="stitches_asc">Least Stitches</option>
                  <option value="title_asc">Title (A to Z)</option>
                  <option value="size_desc">Largest File Size</option>
                </select>
              </div>

              {hasActiveFilters && (
                <button
                  className="secondary compact-btn"
                  onClick={handleResetFilters}
                >
                  Clear Filters
                </button>
              )}

              <span className="spacer" />

              {/* Grid / List View Toggle */}
              <div className="view-toggle-group">
                <button
                  className={`toggle-btn ${isGridView ? "active" : ""}`}
                  onClick={() => setIsGridView(true)}
                  aria-label="Grid view"
                >
                  <Grid2X2 size={17} />
                </button>
                <button
                  className={`toggle-btn ${!isGridView ? "active" : ""}`}
                  onClick={() => setIsGridView(false)}
                  aria-label="List view"
                >
                  <LayoutList size={17} />
                </button>
              </div>
            </div>

            {/* Format Filter Pill Bar */}
            <div className="format-pills-bar">
              {FORMAT_OPTIONS.map((fmt) => (
                <button
                  key={fmt}
                  className={`format-pill ${selectedFormat === fmt ? "active" : ""}`}
                  onClick={() => setSelectedFormat(fmt)}
                >
                  {fmt.toUpperCase()}
                </button>
              ))}
            </div>

            {/* Main Design Catalog Content */}
            {loading && designs.length === 0 ? (
              <div className="empty-box">
                <RefreshCw size={32} className="spin text-accent" />
                <p>Loading design library…</p>
              </div>
            ) : designs.length === 0 ? (
              <div className="empty-box">
                <ArchiveRestore size={38} />
                <h3>No embroidery designs found</h3>
                <p>
                  {hasActiveFilters
                    ? "Try loosening your search query or format filters."
                    : "Drag and drop or import DST, PES, JEF, or other embroidery files to begin."}
                </p>
                {hasActiveFilters ? (
                  <button
                    className="secondary mt-3"
                    onClick={handleResetFilters}
                  >
                    Reset all filters
                  </button>
                ) : (
                  <button
                    className="primary mt-3"
                    onClick={() => setIsImportModalOpen(true)}
                  >
                    Import Designs
                  </button>
                )}
              </div>
            ) : isGridView ? (
              /* Grid Layout */
              <div className="design-grid">
                {designs.map((d) => (
                  <article
                    key={d.id}
                    className={`design-card ${selectedDesign?.id === d.id ? "selected" : ""}`}
                    onClick={() => setSelectedDesign(d)}
                  >
                    <DesignImage
                      previewPath={d.previewPath}
                      title={d.title}
                      format={d.format}
                    />
                    <div className="card-info">
                      <div>
                        <h3>{d.title}</h3>
                        <p className="text-xs text-subtle truncate">{d.filename}</p>
                      </div>
                      <span className="format-badge">{d.format}</span>
                    </div>

                    <div className="design-facts">
                      <span>
                        {d.widthMm ? `${d.widthMm} × ${d.heightMm} mm` : "—"}
                      </span>
                      <span>{d.stitches?.toLocaleString() ?? "—"} stitches</span>
                    </div>

                    {d.tags && d.tags.length > 0 && (
                      <div className="tag-row">
                        {d.tags.slice(0, 3).map((t) => (
                          <span key={t} className="tag-chip">
                            #{t}
                          </span>
                        ))}
                        {d.tags.length > 3 && (
                          <span className="tag-more">+{d.tags.length - 3}</span>
                        )}
                      </div>
                    )}
                  </article>
                ))}
              </div>
            ) : (
              /* List / Table Layout */
              <div className="design-table-container">
                <table className="design-table">
                  <thead>
                    <tr>
                      <th style={{ width: 60 }}>Preview</th>
                      <th>Title & Filename</th>
                      <th>Format</th>
                      <th>Dimensions</th>
                      <th>Stitches</th>
                      <th>Colors</th>
                      <th>Tags</th>
                      <th>Imported</th>
                    </tr>
                  </thead>
                  <tbody>
                    {designs.map((d) => (
                      <tr
                        key={d.id}
                        className={selectedDesign?.id === d.id ? "selected-row" : ""}
                        onClick={() => setSelectedDesign(d)}
                      >
                        <td>
                          <div className="table-thumb">
                            <DesignImage
                              previewPath={d.previewPath}
                              title={d.title}
                              format={d.format}
                            />
                          </div>
                        </td>
                        <td>
                          <b className="block">{d.title}</b>
                          <span className="text-xs text-subtle">{d.filename}</span>
                        </td>
                        <td>
                          <span className="format-badge">{d.format}</span>
                        </td>
                        <td className="text-sm">
                          {d.widthMm ? `${d.widthMm} × ${d.heightMm} mm` : "—"}
                        </td>
                        <td className="text-sm font-mono">
                          {d.stitches?.toLocaleString() ?? "—"}
                        </td>
                        <td className="text-sm">{d.colors ?? "—"}</td>
                        <td>
                          <div className="tag-row">
                            {d.tags.slice(0, 2).map((t) => (
                              <span key={t} className="tag-chip">
                                #{t}
                              </span>
                            ))}
                          </div>
                        </td>
                        <td className="text-xs text-subtle">
                          {new Date(d.importedAt).toLocaleDateString()}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}
      </section>

      {/* Selected Design Details Inspector Drawer */}
      {selectedDesign && (
        <DesignDetailsDrawer
          designId={selectedDesign.id}
          onClose={() => setSelectedDesign(null)}
          onRefreshCatalog={reloadData}
          onTriggerAi={(d) => setAiTargetDesign(d)}
          onDeleteDesign={(id) => {
            void deleteDesign(id).then(() => {
              setSelectedDesign(null);
              void reloadData();
            });
          }}
          onRestoreDesign={(id) => {
            void restoreDesign(id).then(() => {
              setSelectedDesign(null);
              void reloadData();
            });
          }}
        />
      )}

      {/* Batch Import Dialog */}
      <BatchImportModal
        isOpen={isImportModalOpen}
        onClose={() => setIsImportModalOpen(false)}
        onImportComplete={reloadData}
      />

      {/* AI Review & Analysis Modal */}
      <AiReviewModal
        design={aiTargetDesign}
        aiConfig={aiConfig}
        isOpen={Boolean(aiTargetDesign)}
        onClose={() => setAiTargetDesign(null)}
        onApplied={reloadData}
        onOpenSettings={() => {
          setAiTargetDesign(null);
          setSection("settings");
        }}
      />

    </main>
  );
}

function NavItem({
  icon,
  label,
  count,
  active,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  count?: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`nav-item ${active ? "active" : ""}`}
      onClick={onClick}
    >
      <span className="nav-icon">{icon}</span>
      <span className="nav-label">{label}</span>
      {count !== undefined && count > 0 && (
        <span className="nav-count-badge">{count}</span>
      )}
    </button>
  );
}
