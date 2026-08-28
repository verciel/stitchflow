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
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  RefreshCw,
  Search,
  Settings2,
  Sparkles,
  Tag as TagIcon,
  Trash2,
  X,
  Zap,
} from "lucide-react";
import {
  deleteDesign,
  getAiConfig,
  listCollections,
  listDesigns,
  listJobs,
  listTags,
  naturalLanguageSearch,
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

export const App: React.FC = () => {
  const [section, setSection] = useState<Section>("library");
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);

  // Catalog State
  const [designs, setDesigns] = useState<Design[]>([]);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [recycledDesigns, setRecycledDesigns] = useState<Design[]>([]);

  // Filter State
  const [query, setQuery] = useState("");
  const [isAiSearchActive, setIsAiSearchActive] = useState(false);
  const [selectedFormat, setSelectedFormat] = useState<string>("all");
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

  const handleSearchChange = async (val: string) => {
    setQuery(val);
    if (isAiSearchActive && val.trim().length > 3) {
      try {
        const parsed = await naturalLanguageSearch(val);
        if (parsed.format && parsed.format !== selectedFormat) {
          setSelectedFormat(parsed.format);
        }
      } catch (err) {
        console.error("AI natural search error:", err);
      }
    }
  };

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
      <aside className={`sidebar ${isSidebarCollapsed ? "collapsed" : ""}`}>
        <div className="brand">
          <div className="brand-mark">S</div>
          <div className="brand-text">
            <span className="brand-title">Stitchflow</span>
            <span className="brand-subtitle">V1 Desktop Edition</span>
          </div>
          <button
            className="sidebar-collapse-btn"
            onClick={() => setIsSidebarCollapsed(!isSidebarCollapsed)}
            title={isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            {isSidebarCollapsed ? (
              <PanelLeftOpen size={16} />
            ) : (
              <PanelLeftClose size={16} />
            )}
          </button>
        </div>

        <button
          className="import-button"
          onClick={() => setIsImportModalOpen(true)}
          title="Import embroidery designs or artwork"
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
            collapsed={isSidebarCollapsed}
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
            collapsed={isSidebarCollapsed}
            onClick={() => setSection("collections")}
          />
          <NavItem
            icon={<Boxes size={18} />}
            label="Jobs"
            count={jobs.length}
            active={section === "jobs"}
            collapsed={isSidebarCollapsed}
            onClick={() => setSection("jobs")}
          />
          <NavItem
            icon={<ImageIcon size={18} />}
            label="Artwork"
            active={section === "artwork"}
            collapsed={isSidebarCollapsed}
            onClick={() => setSection("artwork")}
          />
          <NavItem
            icon={<Trash2 size={18} />}
            label="Recycle area"
            count={recycledDesigns.length > 0 ? recycledDesigns.length : undefined}
            active={section === "recycle"}
            collapsed={isSidebarCollapsed}
            onClick={() => setSection("recycle")}
          />
        </nav>

        <div className="sidebar-bottom">
          <NavItem
            icon={<Settings2 size={18} />}
            label="Settings"
            active={section === "settings"}
            collapsed={isSidebarCollapsed}
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
                ? "Design Collections"
                : section === "jobs"
                ? "Production Jobs"
                : section === "artwork"
                ? "Source Artwork Assets"
                : section === "recycle"
                ? "Recycle Area"
                : "Settings & Configuration"}
            </h1>
            <p className="subtle">
              {section === "library"
                ? `${designs.length} designs indexed · Secure local storage`
                : section === "collections"
                ? "Organize embroidery designs into themed series and seasonal folders."
                : section === "jobs"
                ? "Production batch containers linking garments, hoop sizes, and customer artwork."
                : section === "artwork"
                ? "Manage customer mockups, sketches, and vector assets (PNG, JPG, SVG, PDF)."
                : section === "recycle"
                ? "Safely restore quarantined files or permanently purge deleted designs."
                : "Configure vision-capable AI endpoints, Ink/Stitch handoff paths, and backup archives."}
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
                title="Run Vision AI analysis on selected design"
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
                  placeholder={
                    isAiSearchActive
                      ? "Natural search (e.g., 'red florals for towels', 'PES under 5000 sts')…"
                      : "Search by title, filename, tags, or AI category…"
                  }
                  value={query}
                  onChange={(e) => void handleSearchChange(e.target.value)}
                />
                <button
                  type="button"
                  className={`ai-search-toggle-btn ${isAiSearchActive ? "active" : ""}`}
                  onClick={() => setIsAiSearchActive(!isAiSearchActive)}
                  title="Toggle Natural Language AI Search"
                >
                  <Sparkles size={13} />
                  <span>AI</span>
                </button>
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

              {/* Reset Filters */}
              {hasActiveFilters && (
                <button
                  className="text-button text-sm"
                  onClick={handleResetFilters}
                >
                  Clear filters
                </button>
              )}

              <div className="spacer" />

              {/* Grid / Table View Toggle */}
              <div className="view-toggle-group">
                <button
                  className={`toggle-btn ${isGridView ? "active" : ""}`}
                  onClick={() => setIsGridView(true)}
                  title="Grid view"
                >
                  <Grid2X2 size={17} />
                </button>
                <button
                  className={`toggle-btn ${!isGridView ? "active" : ""}`}
                  onClick={() => setIsGridView(false)}
                  title="Table view"
                >
                  <LayoutList size={17} />
                </button>
              </div>
            </div>

            {/* Format Pills Bar */}
            <div className="format-pills-bar">
              {FORMAT_OPTIONS.map((fmt) => (
                <button
                  key={fmt}
                  className={`format-pill ${selectedFormat === fmt ? "active" : ""}`}
                  onClick={() => setSelectedFormat(fmt)}
                >
                  {fmt === "all" ? "All Formats" : fmt}
                </button>
              ))}
            </div>

            {/* Active Collection or Job Banner */}
            {(activeCollectionName || activeJobName) && (
              <div className="active-scope-banner">
                <span>
                  Showing designs scoped to:{" "}
                  <b>{activeCollectionName || activeJobName}</b>
                </span>
                <button
                  className="text-button text-xs"
                  onClick={() => {
                    setSelectedCollectionId(null);
                    setSelectedJobId(null);
                  }}
                >
                  Show all designs
                </button>
              </div>
            )}

            {/* Design Catalog Content */}
            {designs.length === 0 ? (
              <div className="empty-box">
                <Filter size={36} />
                <h3>No embroidery designs found</h3>
                <p>
                  {hasActiveFilters
                    ? "Try adjusting your search query, format, or tag filters."
                    : "Import DST, PES, JEF, VP3, EXP, HUS, XXX, SEW, PCS, PEC files to get started."}
                </p>
                {hasActiveFilters ? (
                  <button className="secondary mt-3" onClick={handleResetFilters}>
                    Reset filters
                  </button>
                ) : (
                  <button
                    className="primary mt-3"
                    onClick={() => setIsImportModalOpen(true)}
                  >
                    <Plus size={16} /> Import Designs
                  </button>
                )}
              </div>
            ) : isGridView ? (
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
                    <div className="design-card-content">
                      <div className="design-card-header">
                        <h4 className="design-title truncate" title={d.title}>
                          {d.title}
                        </h4>
                        <span className="format-badge">{d.format}</span>
                      </div>
                      <p className="design-filename truncate">{d.filename}</p>

                      <div className="design-stats">
                        <span>
                          {d.widthMm && d.heightMm
                            ? `${d.widthMm.toFixed(0)} × ${d.heightMm.toFixed(0)} mm`
                            : "—"}
                        </span>
                        <span>{d.stitches ? `${d.stitches.toLocaleString()} sts` : "—"}</span>
                      </div>

                      {/* Tag preview chips */}
                      {d.tags.length > 0 && (
                        <div className="design-card-tags">
                          {d.tags.slice(0, 3).map((t) => (
                            <span key={t} className="card-tag-pill">
                              #{t}
                            </span>
                          ))}
                          {d.tags.length > 3 && (
                            <span className="card-tag-more">
                              +{d.tags.length - 3}
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  </article>
                ))}
              </div>
            ) : (
              /* Table View */
              <div className="table-container">
                <table className="design-table">
                  <thead>
                    <tr>
                      <th>Title</th>
                      <th>Format</th>
                      <th>Dimensions</th>
                      <th>Stitches</th>
                      <th>Colors</th>
                      <th>Tags</th>
                      <th>Size</th>
                      <th>Date</th>
                    </tr>
                  </thead>
                  <tbody>
                    {designs.map((d) => (
                      <tr
                        key={d.id}
                        className={selectedDesign?.id === d.id ? "selected" : ""}
                        onClick={() => setSelectedDesign(d)}
                      >
                        <td>
                          <b>{d.title}</b>
                          <small className="block text-subtle">{d.filename}</small>
                        </td>
                        <td>
                          <span className="format-badge">{d.format}</span>
                        </td>
                        <td>
                          {d.widthMm && d.heightMm
                            ? `${d.widthMm.toFixed(1)} × ${d.heightMm.toFixed(1)} mm`
                            : "—"}
                        </td>
                        <td>{d.stitches ? d.stitches.toLocaleString() : "—"}</td>
                        <td>{d.colors ?? "—"}</td>
                        <td>
                          {d.tags.length > 0 ? (
                            <div className="flex gap-1 flex-wrap">
                              {d.tags.slice(0, 2).map((t) => (
                                <span key={t} className="card-tag-pill text-xs">
                                  #{t}
                                </span>
                              ))}
                              {d.tags.length > 2 && (
                                <small>+{d.tags.length - 2}</small>
                              )}
                            </div>
                          ) : (
                            "—"
                          )}
                        </td>
                        <td>{(d.sizeBytes / 1024).toFixed(1)} KB</td>
                        <td className="text-subtle text-xs">
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

      {/* Design Inspector Drawer */}
      {selectedDesign && (
        <DesignDetailsDrawer
          designId={selectedDesign.id}
          onClose={() => setSelectedDesign(null)}
          onRefreshCatalog={reloadData}
          onTriggerAi={(d) => setAiTargetDesign(d)}
          onDeleteDesign={async (id) => {
            try {
              await deleteDesign(id);
              setSelectedDesign(null);
              await reloadData();
            } catch (err) {
              console.error(err);
            }
          }}
          onRestoreDesign={async (id) => {
            try {
              await restoreDesign(id);
              await reloadData();
            } catch (err) {
              console.error(err);
            }
          }}
        />
      )}

      {/* Batch Import Dropzone Modal */}
      <BatchImportModal
        isOpen={isImportModalOpen}
        onClose={() => setIsImportModalOpen(false)}
        onImportComplete={reloadData}
      />

      {/* AI Review & Classification Modal */}
      {aiTargetDesign && (
        <AiReviewModal
          isOpen={Boolean(aiTargetDesign)}
          design={aiTargetDesign}
          aiConfig={aiConfig}
          onClose={() => setAiTargetDesign(null)}
          onApplied={() => {
            void reloadData();
            setAiTargetDesign(null);
          }}
          onOpenSettings={() => {
            setAiTargetDesign(null);
            setSection("settings");
          }}
        />
      )}

    </main>
  );
};

interface NavItemProps {
  icon: React.ReactNode;
  label: string;
  count?: number;
  active?: boolean;
  collapsed?: boolean;
  onClick: () => void;
}

const NavItem: React.FC<NavItemProps> = ({
  icon,
  label,
  count,
  active,
  collapsed,
  onClick,
}) => (
  <button
    className={`nav-item ${active ? "active" : ""}`}
    onClick={onClick}
    title={collapsed ? `${label}${count !== undefined ? ` (${count})` : ""}` : undefined}
  >
    {icon}
    {!collapsed && (
      <>
        <span className="nav-label">{label}</span>
        {count !== undefined && (
          <span className="nav-count-badge">{count}</span>
        )}
      </>
    )}
  </button>
);
