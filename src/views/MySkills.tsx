import { useEffect, useMemo, useRef, useState } from "react";
import {
  Search,
  LayoutGrid,
  List,
  CheckCircle2,
  Circle,
  Github,
  HardDrive,
  Globe,
  Trash2,
  Layers,
  RefreshCw,
  RotateCcw,
  GitBranch,
  ArrowUpCircle,
  Loader2,
  X,
  Plus,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { cn } from "../utils";
import { useApp } from "../context/AppContext";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { SkillDetailPanel } from "../components/SkillDetailPanel";
import * as api from "../lib/tauri";
import type {
  ManagedSkill,
  MigrationScanResult,
  ToolInfo,
  GitBackupStatus,
  OriginResolutionPlan,
} from "../lib/tauri";
import { getErrorMessage } from "../lib/error";

function getToolDisplayName(toolKey: string, tools: ToolInfo[]) {
  return tools.find((tool) => tool.key === toolKey)?.display_name || toolKey;
}

export function MySkills() {
  const { t } = useTranslation();
  const {
    activeScenario,
    tools,
    managedSkills: skills,
    refreshScenarios,
    refreshManagedSkills,
    detailSkillId,
    openSkillDetailById,
    closeSkillDetail,
  } = useApp();
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
  const [filterMode, setFilterMode] = useState<"all" | "enabled" | "available">("all");
  const [sourceFilters, setSourceFilters] = useState<Set<string>>(new Set());
  const [tagFilters, setTagFilters] = useState<Set<string>>(new Set());
  const [allTags, setAllTags] = useState<string[]>([]);
  const [search, setSearch] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<ManagedSkill | null>(null);
  const [syncingSkillId, setSyncingSkillId] = useState<string | null>(null);
  const [checkingAll, setCheckingAll] = useState(false);
  const [checkingSkillId, setCheckingSkillId] = useState<string | null>(null);
  const [updatingSkillId, setUpdatingSkillId] = useState<string | null>(null);
  const [gitStatus, setGitStatus] = useState<GitBackupStatus | null>(null);
  const [gitLoading, setGitLoading] = useState<string | null>(null); // "start" | "sync"
  const [gitRemoteConfig, setGitRemoteConfig] = useState("");
  const [tagEditSkillId, setTagEditSkillId] = useState<string | null>(null);
  const [tagInput, setTagInput] = useState("");
  const [originAuditPlan, setOriginAuditPlan] = useState<OriginResolutionPlan | null>(null);
  const [originAuditOpen, setOriginAuditOpen] = useState(false);
  const [originAuditLoading, setOriginAuditLoading] = useState(false);
  const [originBackfillLoading, setOriginBackfillLoading] = useState(false);
  const [runtimePlacementPlan, setRuntimePlacementPlan] = useState<{
    codex: MigrationScanResult | null;
    global: MigrationScanResult | null;
  } | null>(null);
  const [runtimePlacementOpen, setRuntimePlacementOpen] = useState(false);
  const [runtimePlacementLoading, setRuntimePlacementLoading] = useState(false);
  const tagInputRef = useRef<HTMLInputElement>(null);

  const installedTools = tools.filter((tool) => tool.installed);
  const activeScenarioName = activeScenario?.name || t("mySkills.currentScenarioFallback");

  const enabledCount = activeScenario
    ? skills.filter((skill) => skill.scenario_ids.includes(activeScenario.id)).length
    : 0;

  const refreshAllTags = async () => {
    try {
      const tags = await api.getAllTags();
      setAllTags(tags);
    } catch {
      // not critical
    }
  };

  useEffect(() => {
    refreshAllTags();
  }, [skills]);

  const toggleFilter = (set: Set<string>, value: string): Set<string> => {
    const next = new Set(set);
    if (next.has(value)) next.delete(value);
    else next.add(value);
    return next;
  };

  const filtered = useMemo(() => {
    const result = skills.filter((skill) => {
      const matchesSearch =
        skill.name.toLowerCase().includes(search.toLowerCase()) ||
        (skill.description || "").toLowerCase().includes(search.toLowerCase());
      if (!matchesSearch) return false;

      if (sourceFilters.size > 0 && !sourceFilters.has(skill.source_type)) return false;

      if (tagFilters.size > 0 && !skill.tags.some((t) => tagFilters.has(t))) return false;

      if (!activeScenario) return true;

      const enabledInScenario = skill.scenario_ids.includes(activeScenario.id);
      if (filterMode === "enabled") return enabledInScenario;
      if (filterMode === "available") return !enabledInScenario;
      return true;
    });

    if (activeScenario) {
      result.sort((a, b) => {
        const aEnabled = a.scenario_ids.includes(activeScenario.id) ? 0 : 1;
        const bEnabled = b.scenario_ids.includes(activeScenario.id) ? 0 : 1;
        if (aEnabled !== bEnabled) return aEnabled - bEnabled;
        return a.name.localeCompare(b.name);
      });
    }

    return result;
  }, [skills, search, sourceFilters, tagFilters, filterMode, activeScenario]);

  const selectedSkill = useMemo(
    () => skills.find((skill) => skill.id === detailSkillId) || null,
    [detailSkillId, skills]
  );

  const mapGitError = (error: unknown) => {
    const message = String(error);
    if (
      message.includes("Authentication failed")
      || message.includes("Permission denied")
      || message.includes("could not read Username")
    ) {
      return t("settings.gitErrorAuth");
    }
    if (
      message.includes("Could not resolve host")
      || message.includes("Failed to connect")
      || message.includes("Connection timed out")
    ) {
      return t("settings.gitErrorNetwork");
    }
    if (message.includes("CONFLICT") || message.includes("conflict")) {
      return t("settings.gitErrorConflict");
    }
    if (message.includes("not a git repository")) {
      return t("settings.gitErrorNotRepo");
    }
    return t("settings.gitErrorGeneric");
  };

  const refreshGitStatus = async () => {
    try {
      const status = await api.gitBackupStatus();
      setGitStatus(status);
    } catch {
      // not critical
    }
  };

  useEffect(() => {
    (async () => {
      const savedRemote = (await api.getSettings("git_backup_remote_url").catch(() => null))?.trim() || "";
      const status = await api.gitBackupStatus().catch(() => null);
      setGitStatus(status);

      if (savedRemote) {
        setGitRemoteConfig(savedRemote);
        return;
      }

      const detectedRemote = status?.remote_url?.trim() || "";
      if (detectedRemote) {
        setGitRemoteConfig(detectedRemote);
        api.setSettings("git_backup_remote_url", detectedRemote).catch(() => {});
      }
    })();
  }, []);

  const getSyncMeta = (skill: ManagedSkill) => {
    const syncedToolKeys = skill.targets
      .map((target) => target.tool)
      .filter((toolKey, index, values) => values.indexOf(toolKey) === index);
    const syncedToolLabels = syncedToolKeys.map((toolKey) => getToolDisplayName(toolKey, tools));
    const pendingTools = installedTools.filter((tool) => !syncedToolKeys.includes(tool.key));

    return {
      syncedToolKeys,
      syncedToolLabels,
      pendingToolKeys: pendingTools.map((tool) => tool.key),
      pendingToolLabels: pendingTools.map((tool) => tool.display_name),
    };
  };

  const handleSyncAction = async (skill: ManagedSkill, mode: "sync" | "unsync") => {
    const syncMeta = getSyncMeta(skill);
    const toolKeys = mode === "sync" ? syncMeta.pendingToolKeys : syncMeta.syncedToolKeys;

    if (toolKeys.length === 0) {
      toast.message(
        mode === "sync" ? t("mySkills.syncNothingToDo") : t("mySkills.unsyncNothingToDo")
      );
      return;
    }
    
    setSyncingSkillId(skill.id);
    try {
      for (const toolKey of toolKeys) {
        if (mode === "sync") {
          await api.syncSkillToTool(skill.id, toolKey);
        } else {
          await api.unsyncSkillFromTool(skill.id, toolKey);
        }
      }

      toast.success(
        mode === "sync"
          ? t("mySkills.syncCompleted", {
              name: skill.name,
              count: toolKeys.length,
            })
          : t("mySkills.unsyncCompleted", {
              name: skill.name,
              count: toolKeys.length,
            })
      );
      await refreshManagedSkills();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
      await refreshManagedSkills();
    } finally {
      setSyncingSkillId(null);
    }
  };

  const handleDeleteManagedSkill = async () => {
    if (!deleteTarget) return;
    await api.deleteManagedSkill(deleteTarget.id);
    if (selectedSkill?.id === deleteTarget.id) closeSkillDetail();
    toast.success(`${deleteTarget.name} ${t("mySkills.deleted")}`);
    setDeleteTarget(null);
    await Promise.all([refreshManagedSkills(), refreshScenarios()]);
  };

  const handleToggleScenario = async (skill: ManagedSkill) => {
    if (!activeScenario) return;
    const enabledInScenario = skill.scenario_ids.includes(activeScenario.id);
    if (enabledInScenario) {
      await api.removeSkillFromScenario(skill.id, activeScenario.id);
      toast.success(`${skill.name} ${t("mySkills.disabledInScenario")}`);
    } else {
      await api.addSkillToScenario(skill.id, activeScenario.id);
      toast.success(`${skill.name} ${t("mySkills.enabledInScenario")}`);
    }
    await Promise.all([refreshManagedSkills(), refreshScenarios()]);
  };

  const handleCheckAllUpdates = async () => {
    setCheckingAll(true);
    try {
      await api.checkAllSkillUpdates(true);
      toast.success(t("mySkills.updateActions.checkedAll"));
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      await refreshManagedSkills();
      setCheckingAll(false);
    }
  };

  const handleCheckUpdate = async (skill: ManagedSkill) => {
    setCheckingSkillId(skill.id);
    try {
      await api.checkSkillUpdate(skill.id, true);
      await refreshManagedSkills();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
      await refreshManagedSkills();
    } finally {
      setCheckingSkillId(null);
    }
  };

  const handleRefreshSkill = async (skill: ManagedSkill) => {
    setUpdatingSkillId(skill.id);
    try {
      if (skill.source_type === "local" || skill.source_type === "import") {
        await api.reimportLocalSkill(skill.id);
        toast.success(t("mySkills.updateActions.reimported"));
      } else {
        await api.updateSkill(skill.id);
        toast.success(t("mySkills.updateActions.updated"));
      }
      await refreshManagedSkills();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
      await refreshManagedSkills();
    } finally {
      setUpdatingSkillId(null);
    }
  };

  const handleAuditOrigins = async () => {
    setOriginAuditLoading(true);
    try {
      const plan = await api.scanOriginResolution();
      setOriginAuditPlan(plan);
      setOriginAuditOpen(true);
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setOriginAuditLoading(false);
    }
  };

  const handleApplyOriginAudit = async () => {
    if (!originAuditPlan) return;
    const selectedPaths = originAuditPlan.entries
      .filter((entry) => entry.action === "write-custom-no-source")
      .map((entry) => entry.path);

    if (selectedPaths.length === 0) {
      toast.message(t("mySkills.originAuditNoAction"));
      return;
    }

    setOriginAuditLoading(true);
    try {
      const result = await api.applyOriginResolution(selectedPaths, originAuditPlan.root);
      toast.success(
        t("mySkills.originAuditApplied", {
          count: result.applied,
        })
      );
      if (result.network_review_needed > 0) {
        toast.message(
          t("mySkills.originAuditReview", {
            count: result.network_review_needed,
          })
        );
      }
      await refreshManagedSkills();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setOriginAuditLoading(false);
      setOriginAuditOpen(false);
    }
  };

  const handleApplyOriginBackfill = async () => {
    setOriginBackfillLoading(true);
    try {
      const result = await api.applyOriginBackfill();
      toast.success(
        t("mySkills.sourceBackfillApplied", {
          count: result.applied,
        })
      );
      if (result.review_needed > 0) {
        toast.message(
          t("mySkills.sourceBackfillReview", {
            count: result.review_needed,
          })
        );
      }
      await refreshManagedSkills();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setOriginBackfillLoading(false);
    }
  };

  const handleRuntimePlacementSnapshot = async () => {
    setRuntimePlacementLoading(true);
    try {
      const [codex, global] = await Promise.all([
        api.scanRuntimeMigration("C:\\Users\\Administrator\\.codex\\skills"),
        api.scanRuntimeMigration("C:\\Users\\Administrator\\.agents\\skills"),
      ]);
      setRuntimePlacementPlan({ codex, global });
      setRuntimePlacementOpen(true);
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setRuntimePlacementLoading(false);
    }
  };

  const handleAddTag = async (skill: ManagedSkill, inputValue?: string) => {
    const trimmed = (inputValue ?? tagInput).trim();
    if (!trimmed || skill.tags.includes(trimmed)) {
      setTagInput("");
      return;
    }
    try {
      await api.setSkillTags(skill.id, [...skill.tags, trimmed]);
      toast.success(t("mySkills.tags.tagAdded"));
      setTagEditSkillId(null);
      setTagInput("");
      await refreshManagedSkills();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    }
  };

  const handleRemoveTag = async (skill: ManagedSkill, tagToRemove: string) => {
    try {
      await api.setSkillTags(skill.id, skill.tags.filter((t) => t !== tagToRemove));
      toast.success(t("mySkills.tags.tagsUpdated"));
      await refreshManagedSkills();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    }
  };

  const getTagOptions = (skill: ManagedSkill, keyword: string) => {
    const needle = keyword.trim().toLowerCase();
    return allTags.filter((tag) => {
      if (skill.tags.includes(tag)) return false;
      if (!needle) return true;
      return tag.toLowerCase().includes(needle);
    });
  };

  const handleGitStartBackup = async () => {
    setGitLoading("start");
    try {
      if (gitRemoteConfig) {
        await api.gitBackupClone(gitRemoteConfig);
        toast.success(t("settings.gitCloneSuccess"));
      } else {
        await api.gitBackupInit();
        toast.success(t("settings.gitInitSuccess"));
      }
      await refreshGitStatus();
    } catch (e) {
      toast.error(mapGitError(e));
    } finally {
      setGitLoading(null);
    }
  };

  const handleGitSync = async () => {
    setGitLoading("sync");
    try {
      let status = await api.gitBackupStatus();
      if (!status.is_repo) {
        toast.info(t("settings.gitNotInitialized"));
        return;
      }

      if (!status.remote_url && gitRemoteConfig) {
        await api.gitBackupSetRemote(gitRemoteConfig);
        status = await api.gitBackupStatus();
      }

      if (!status.remote_url) {
        toast.info(t("settings.gitNeedRemoteSetup"));
        return;
      }

      if (status.behind > 0 && (status.has_changes || status.ahead > 0)) {
        toast.info(t("settings.gitNeedPullFirst"));
        return;
      }

      if (status.behind > 0) {
        await api.gitBackupPull();
        status = await api.gitBackupStatus();
        toast.success(t("settings.gitPullSuccess"));
      }

      let committed = false;
      if (status.has_changes) {
        await api.gitBackupCommit(t("settings.gitCommitPlaceholder"));
        committed = true;
      }

      if (committed || status.ahead > 0) {
        await api.gitBackupPush();
        toast.success(t("settings.gitSyncSuccess"));
      } else {
        toast.success(t("settings.gitUpToDate"));
      }

      await refreshGitStatus();
    } catch (e) {
      toast.error(mapGitError(e));
    } finally {
      setGitLoading(null);
    }
  };

  const getGitSyncButtonState = () => {
    if (!gitStatus) {
      return {
        label: t("mySkills.gitRepoSync"),
        disabled: false,
        toneClassName: "text-secondary",
      };
    }
    if (!gitStatus.remote_url && !gitRemoteConfig) {
      return {
        label: t("mySkills.gitRepoNeedRemote"),
        disabled: true,
        toneClassName: "text-red-500",
      };
    }
    if (gitStatus.behind > 0 && (gitStatus.has_changes || gitStatus.ahead > 0)) {
      return {
        label: t("mySkills.gitRepoSync"),
        disabled: false,
        toneClassName: "text-red-500",
      };
    }
    if (gitStatus.has_changes || gitStatus.ahead > 0 || gitStatus.behind > 0) {
      return {
        label: t("mySkills.gitRepoSync"),
        disabled: false,
        toneClassName: "text-amber-500",
      };
    }
    if (!gitStatus.has_changes && gitStatus.ahead === 0 && gitStatus.behind === 0) {
      return {
        label: t("mySkills.gitRepoUpToDate"),
        disabled: true,
        toneClassName: "text-muted",
      };
    }
    return {
      label: t("mySkills.gitRepoSync"),
      disabled: false,
      toneClassName: "text-secondary",
    };
  };

  const sourceIcon = (type: string) => {
    switch (type) {
      case "git":
      case "skillssh":
        return <Github className="h-3 w-3" />;
      case "local":
      case "import":
        return <HardDrive className="h-3 w-3" />;
      default:
        return <Globe className="h-3 w-3" />;
    }
  };

  const canRefresh = (skill: ManagedSkill) =>
    skill.source_type === "git" ||
    skill.source_type === "skillssh" ||
    skill.source_type === "local" ||
    skill.source_type === "import";

  const sourceTypeLabel = (skill: ManagedSkill) =>
    skill.source_type === "skillssh" ? "skills.sh" : skill.source_type;

  const refreshLabel = (skill: ManagedSkill) =>
    skill.source_type === "local" || skill.source_type === "import"
      ? t("mySkills.updateActions.reimport")
      : t("mySkills.updateActions.update");

  const sourceKindMeta = (skill: ManagedSkill) => {
    switch (skill.source_kind) {
      case "verified-upstream":
        return {
          label: t("mySkills.sourceKind.verifiedUpstream"),
          className: "bg-emerald-500/12 text-emerald-600 dark:text-emerald-400",
        };
      case "verified-distribution":
        return {
          label: t("mySkills.sourceKind.verifiedDistribution"),
          className: "bg-sky-500/12 text-sky-600 dark:text-sky-400",
        };
      case "inferred-upstream":
        return {
          label: t("mySkills.sourceKind.inferredUpstream"),
          className: "bg-amber-500/12 text-amber-600 dark:text-amber-400",
        };
      case "custom-no-source":
        return {
          label: t("mySkills.sourceKind.customNoSource"),
          className: "bg-slate-500/12 text-slate-600 dark:text-slate-300",
        };
      case "replaced-equivalent":
        return {
          label: t("mySkills.sourceKind.replacedEquivalent"),
          className: "bg-violet-500/12 text-violet-600 dark:text-violet-400",
        };
      case "system-reserved":
        return {
          label: t("mySkills.sourceKind.systemReserved"),
          className: "bg-rose-500/12 text-rose-600 dark:text-rose-300",
        };
      default:
        return null;
    }
  };

  const updateStatusMeta = (skill: ManagedSkill) => {
    const status = skill.update_status;
    if (status === "update-available" || status === "update_available") {
      return {
        label: t("mySkills.updateStatus.updateAvailable"),
        className: "bg-amber-500/12 text-amber-600 dark:text-amber-400",
      };
    }
    if (status === "reimport-available" || status === "source_missing") {
      return {
        label: t("mySkills.updateStatus.reimportAvailable"),
        className: "bg-blue-500/12 text-blue-600 dark:text-blue-400",
      };
    }
    if (status === "legacy-untracked") {
      return {
        label: t("mySkills.updateStatus.legacyUntracked"),
        className: "bg-fuchsia-500/12 text-fuchsia-600 dark:text-fuchsia-400",
      };
    }
    if (status === "policy-blocked") {
      return {
        label: t("mySkills.updateStatus.policyBlocked"),
        className: "bg-rose-500/12 text-rose-600 dark:text-rose-300",
      };
    }
    if (status === "check-failed" || status === "error") {
      return {
        label: t("mySkills.updateStatus.checkFailed"),
        className: "bg-red-500/10 text-red-600 dark:text-red-300",
      };
    }
    return null;
  };

  const sourceSummary = (skill: ManagedSkill) => {
    if (skill.source_subpath) {
      return `${skill.source_ref || "—"}#${skill.source_subpath}`;
    }
    return skill.source_ref || "—";
  };

  const revisionSummary = (skill: ManagedSkill) => {
    if (skill.source_revision && skill.remote_revision && skill.source_revision !== skill.remote_revision) {
      return `${skill.source_revision.slice(0, 7)} → ${skill.remote_revision.slice(0, 7)}`;
    }
    if (skill.source_revision) return skill.source_revision.slice(0, 7);
    if (skill.remote_revision) return skill.remote_revision.slice(0, 7);
    return null;
  };

  const statusBadge = (skill: ManagedSkill) => {
    return updateStatusMeta(skill);
  };

  const summarizePlacement = (result: MigrationScanResult | null) => {
    const entries = result?.entries || [];
    const counts = entries.reduce(
      (acc, entry) => {
        if (entry.placement_kind === "codex-system") acc.system += 1;
        else if (entry.placement_kind === "shared-junction") acc.shared += 1;
        else if (entry.placement_kind === "broken-link") acc.broken += 1;
        else if (entry.placement_kind === "custom-local") acc.local += 1;
        else acc.unknown += 1;
        return acc;
      },
      { system: 0, shared: 0, broken: 0, local: 0, unknown: 0 }
    );
    const samples = entries.slice(0, 4).map((entry) => {
      const placement = entry.placement_kind || "unknown";
      const source = entry.source_kind || "unknown";
      return `${entry.name} · ${placement} · ${source}`;
    });
    return { counts, samples };
  };

  return (
    <div className="app-page">
      <div className="app-page-header pr-2">
        <h1 className="app-page-title flex items-center gap-2.5">
          {t("mySkills.title")}
          <span className="app-badge">
            {skills.length}
          </span>
        </h1>
        <p className="app-page-subtitle">
          {activeScenario
            ? t("mySkills.subtitle", { scenario: activeScenario.name, count: enabledCount })
            : t("mySkills.noScenario")}
        </p>
      </div>

      <div className="app-toolbar">
        <div className="flex flex-1 gap-3">
          <div className="relative w-full max-w-[280px]">
            <Search className="absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={t("mySkills.searchPlaceholder")}
              className="app-input w-full pl-9 font-medium"
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
            />
          </div>

          <div className="app-segmented">
            {(["all", "enabled", "available"] as const).map((mode) => (
              <button
                key={mode}
                onClick={() => setFilterMode(mode)}
                className={cn(
                  "app-segmented-button",
                  filterMode === mode && "app-segmented-button-active"
                )}
              >
                {t(`mySkills.filters.${mode}`)}
              </button>
            ))}
          </div>

        </div>

        <div className="app-segmented">
          {!gitStatus?.is_repo ? (
            <button
              onClick={handleGitStartBackup}
              disabled={!!gitLoading}
              className="inline-flex items-center gap-1 rounded-md px-3 py-2 text-[13px] font-medium text-muted transition-colors hover:bg-surface-hover hover:text-secondary disabled:opacity-50"
            >
              {gitLoading === "start" ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <GitBranch className="h-3.5 w-3.5" />
              )}
              {gitLoading === "start" ? t("settings.gitInitializing") : t("settings.gitStartBackup")}
            </button>
          ) : (
            (() => {
              const gitSyncButton = getGitSyncButtonState();
              return (
                <button
                  onClick={handleGitSync}
                  disabled={!!gitLoading || gitSyncButton.disabled}
                  className={cn(
                    "inline-flex items-center gap-1 rounded-md px-3 py-2 text-[13px] font-medium transition-colors hover:bg-surface-hover disabled:opacity-50",
                    gitSyncButton.toneClassName
                  )}
                >
                  {gitLoading === "sync" ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <ArrowUpCircle className="h-3.5 w-3.5" />
                  )}
                  {gitLoading === "sync" ? t("mySkills.gitRepoSyncing") : gitSyncButton.label}
                </button>
              );
            })()
          )}
          <button
            onClick={handleCheckAllUpdates}
            disabled={checkingAll}
            className="ml-2 mr-2 inline-flex items-center gap-1 rounded-md border-l border-border-subtle pl-4 pr-3 py-2 text-[13px] font-medium text-muted transition-colors hover:bg-surface-hover hover:text-secondary disabled:opacity-50"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", checkingAll && "animate-spin")} />
            {t("mySkills.updateActions.checkAll")}
          </button>
          <button
            onClick={handleAuditOrigins}
            disabled={originAuditLoading}
            className="inline-flex items-center gap-1 rounded-md px-3 py-2 text-[13px] font-medium text-muted transition-colors hover:bg-surface-hover hover:text-secondary disabled:opacity-50"
          >
            <Layers className={cn("h-3.5 w-3.5", originAuditLoading && "animate-spin")} />
            {originAuditLoading ? t("mySkills.originAuditRunning") : t("mySkills.sourceAudit")}
          </button>
          <button
            onClick={handleApplyOriginBackfill}
            disabled={originBackfillLoading}
            className="inline-flex items-center gap-1 rounded-md px-3 py-2 text-[13px] font-medium text-muted transition-colors hover:bg-surface-hover hover:text-secondary disabled:opacity-50"
          >
            <Search className={cn("h-3.5 w-3.5", originBackfillLoading && "animate-spin")} />
            {originBackfillLoading
              ? t("mySkills.sourceBackfillRunning")
              : t("mySkills.sourceBackfill")}
          </button>
          <button
            onClick={handleRuntimePlacementSnapshot}
            disabled={runtimePlacementLoading}
            className="inline-flex items-center gap-1 rounded-md px-3 py-2 text-[13px] font-medium text-muted transition-colors hover:bg-surface-hover hover:text-secondary disabled:opacity-50"
          >
            <Globe className={cn("h-3.5 w-3.5", runtimePlacementLoading && "animate-spin")} />
            {runtimePlacementLoading
              ? t("mySkills.runtimePlacementRunning")
              : t("mySkills.runtimePlacement")}
          </button>
          <button
            onClick={() => setViewMode("grid")}
            className={cn(
              "rounded-md p-2 transition-colors outline-none",
              viewMode === "grid" ? "bg-surface-active text-secondary" : "text-muted hover:text-tertiary"
            )}
          >
            <LayoutGrid className="h-4 w-4" />
          </button>
          <button
            onClick={() => setViewMode("list")}
            className={cn(
              "rounded-md p-2 transition-colors outline-none",
              viewMode === "list" ? "bg-surface-active text-secondary" : "text-muted hover:text-tertiary"
            )}
          >
            <List className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-1 px-1 -mt-4 -mb-4">
        {(["local", "import", "git", "skillssh"] as const).map((src) => (
          <button
            key={src}
            onClick={() => setSourceFilters(toggleFilter(sourceFilters, src))}
            className={cn(
              "rounded-full px-2.5 py-0.5 text-[12px] font-medium transition-colors",
              sourceFilters.has(src)
                ? "bg-accent text-white dark:bg-accent dark:text-white"
                : "bg-surface-hover text-muted hover:text-secondary"
            )}
          >
            {t(`mySkills.sourceFilter.${src}`)}
          </button>
        ))}
        {allTags.length > 0 && (
          <>
            <span className="mx-0.5 h-3 w-px bg-border-subtle" />
            {allTags.map((tag, i) => {
              const colors = [
                "bg-blue-500/15 text-blue-600 dark:text-blue-400",
                "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
                "bg-violet-500/15 text-violet-600 dark:text-violet-400",
                "bg-amber-500/15 text-amber-600 dark:text-amber-400",
                "bg-rose-500/15 text-rose-600 dark:text-rose-400",
                "bg-cyan-500/15 text-cyan-600 dark:text-cyan-400",
                "bg-orange-500/15 text-orange-600 dark:text-orange-400",
                "bg-pink-500/15 text-pink-600 dark:text-pink-400",
              ];
              const activeColors = [
                "bg-blue-500 text-white dark:bg-blue-500",
                "bg-emerald-500 text-white dark:bg-emerald-500",
                "bg-violet-500 text-white dark:bg-violet-500",
                "bg-amber-500 text-white dark:bg-amber-500",
                "bg-rose-500 text-white dark:bg-rose-500",
                "bg-cyan-500 text-white dark:bg-cyan-500",
                "bg-orange-500 text-white dark:bg-orange-500",
                "bg-pink-500 text-white dark:bg-pink-500",
              ];
              const colorIndex = i % colors.length;
              const isActive = tagFilters.has(tag);
              return (
                <button
                  key={tag}
                  onClick={() => setTagFilters(toggleFilter(tagFilters, tag))}
                  className={cn(
                    "rounded-full px-2.5 py-0.5 text-[12px] font-medium transition-colors",
                    isActive ? activeColors[colorIndex] : colors[colorIndex]
                  )}
                >
                  {tag}
                </button>
              );
            })}
          </>
        )}
      </div>

      {filtered.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center pb-20 text-center">
          <Layers className="mb-4 h-12 w-12 text-faint" />
          <h3 className="mb-1.5 text-[14px] font-semibold text-tertiary">{t("mySkills.noSkills")}</h3>
          <p className="text-[13px] text-muted">
            {skills.length === 0 ? t("mySkills.addFirst") : t("mySkills.noMatch")}
          </p>
        </div>
      ) : (
        <div
          className={cn(
            "pb-8",
            viewMode === "grid"
              ? "grid grid-cols-2 gap-3 lg:grid-cols-3"
              : "flex flex-col gap-0.5"
          )}
        >
          {filtered.map((skill) => {
            const isSynced = skill.targets.length > 0;
            const enabledInScenario = activeScenario
              ? skill.scenario_ids.includes(activeScenario.id)
              : false;
            const badge = statusBadge(skill);
            const sourceBadge = sourceKindMeta(skill);

            if (viewMode === "grid") {
              return (
                <div
                  key={skill.id}
                  className={cn(
                    "app-panel group relative flex flex-col overflow-hidden transition-all hover:border-border hover:bg-surface-hover",
                    enabledInScenario && "border-l-2 border-l-accent"
                  )}
                >
                  <div className="absolute right-3 top-3 flex items-center gap-1 opacity-0 transition-all group-hover:opacity-100">
                    <button
                      onClick={() => handleCheckUpdate(skill)}
                      disabled={checkingSkillId === skill.id}
                      className="rounded p-1 text-muted transition-colors hover:bg-surface-hover hover:text-secondary disabled:opacity-50"
                      title={t("mySkills.updateActions.check")}
                    >
                      <RefreshCw className={cn("h-3.5 w-3.5", checkingSkillId === skill.id && "animate-spin")} />
                    </button>
                    {canRefresh(skill) ? (
                      <button
                        onClick={() => handleRefreshSkill(skill)}
                        disabled={updatingSkillId === skill.id}
                        className="rounded p-1 text-accent-light transition-colors hover:bg-accent-bg disabled:opacity-50"
                        title={refreshLabel(skill)}
                      >
                        <RotateCcw className={cn("h-3.5 w-3.5", updatingSkillId === skill.id && "animate-spin")} />
                      </button>
                    ) : null}
                    <button
                      onClick={() => setDeleteTarget(skill)}
                      className="rounded p-1 text-faint transition-colors hover:text-red-400"
                      title={t("mySkills.delete")}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>

                  <div className="flex items-center gap-2.5 px-3.5 pt-3 pb-1.5">
                    {isSynced ? (
                      <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-emerald-500" />
                    ) : (
                      <Circle className="h-3.5 w-3.5 shrink-0 text-faint" />
                    )}
                    <h3
                      className="flex-1 cursor-pointer truncate text-[14px] font-semibold text-primary hover:text-accent-light"
                      onClick={() => openSkillDetailById(skill.id)}
                      title={skill.name}
                    >
                      {skill.name}
                    </h3>
                  </div>

                  <div className="px-3.5 pb-3">
                    <p className="text-[13px] leading-[18px] text-muted truncate">
                      {skill.description || "—"}
                    </p>
                    <p className="mt-1 truncate font-mono text-[11px] text-faint" title={sourceSummary(skill)}>
                      {sourceSummary(skill)}
                    </p>
                    {revisionSummary(skill) && (
                      <p className="truncate font-mono text-[11px] text-faint">
                        {t("mySkills.revision")}: {revisionSummary(skill)}
                      </p>
                    )}
                    {badge && (
                      <div className="mt-2 flex flex-wrap items-center gap-1.5">
                        <span
                          className={cn(
                            "rounded-full px-2 py-0.5 text-[13px] font-medium",
                            badge.className
                          )}
                        >
                          {badge.label}
                        </span>
                        {skill.last_check_error && (
                          <span className="max-w-[220px] truncate text-[11px] text-faint" title={skill.last_check_error}>
                            {skill.last_check_error}
                          </span>
                        )}
                      </div>
                    )}
                    {sourceBadge && (
                      <div className="mt-2 flex flex-wrap items-center gap-1.5">
                        <span className={cn("rounded-full px-2 py-0.5 text-[11px] font-medium", sourceBadge.className)}>
                          {sourceBadge.label}
                        </span>
                        {skill.confidence && (
                          <span className="text-[11px] uppercase tracking-wide text-faint">
                            {skill.confidence}
                          </span>
                        )}
                      </div>
                    )}
                    <div className="mt-2 flex flex-wrap items-center gap-1">
                      {skill.tags.map((tag) => (
                        <span
                          key={tag}
                          className="group/tag inline-flex items-center gap-0.5 rounded-full bg-accent-bg px-2 py-0.5 text-[11px] font-medium text-accent-light"
                        >
                          {tag}
                          <button
                            onClick={(e) => { e.stopPropagation(); handleRemoveTag(skill, tag); }}
                            className="hidden group-hover/tag:inline-flex rounded-full p-0 text-accent-light/60 hover:text-accent-light"
                          >
                            <X className="h-2.5 w-2.5" />
                          </button>
                        </span>
                      ))}
                      {tagEditSkillId === skill.id ? (
                        <div className="relative">
                          <input
                            ref={tagInputRef}
                            type="text"
                            value={tagInput}
                            onChange={(e) => setTagInput(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") { handleAddTag(skill); }
                              if (e.key === "Escape") { setTagEditSkillId(null); setTagInput(""); }
                            }}
                            onBlur={() => {
                              if (tagInput.trim()) handleAddTag(skill);
                              else { setTagEditSkillId(null); setTagInput(""); }
                            }}
                            placeholder={t("mySkills.tags.addTag")}
                            className="h-5 w-28 rounded-full border border-border-subtle bg-transparent px-1.5 text-[11px] text-secondary outline-none focus:border-accent"
                            autoFocus
                          />
                          {getTagOptions(skill, tagInput).length > 0 && (
                            <div className="absolute left-0 top-6 z-10 min-w-[112px] max-w-[180px] rounded-md border border-border-subtle bg-surface p-1 shadow-lg">
                              {getTagOptions(skill, tagInput).slice(0, 6).map((tagOption) => (
                                <button
                                  key={tagOption}
                                  type="button"
                                  onMouseDown={(e) => e.preventDefault()}
                                  onClick={() => handleAddTag(skill, tagOption)}
                                  className="w-full truncate rounded px-1.5 py-1 text-left text-[11px] text-secondary hover:bg-surface-hover"
                                  title={tagOption}
                                >
                                  {tagOption}
                                </button>
                              ))}
                            </div>
                          )}
                        </div>
                      ) : (
                        <button
                          onClick={(e) => { e.stopPropagation(); setTagEditSkillId(skill.id); setTagInput(""); }}
                          className="inline-flex items-center rounded-full p-0.5 text-faint transition-colors hover:text-muted opacity-0 group-hover:opacity-100"
                          title={t("mySkills.tags.addTag")}
                        >
                          <Plus className="h-3 w-3" />
                        </button>
                      )}
                    </div>
                  </div>

                  <div className="mt-auto flex items-center justify-between gap-2 border-t border-border-subtle px-3.5 py-2.5">
                    <div className="flex min-w-0 items-center gap-1.5">
                      <span className="inline-flex shrink-0 items-center gap-1 text-[13px] text-muted">
                        {sourceIcon(skill.source_type)}
                        {sourceTypeLabel(skill)}
                      </span>
                      {enabledInScenario && (
                        <>
                          <span className="text-faint">·</span>
                          <span className="truncate text-[13px] font-medium text-amber-600 dark:text-amber-400/80">
                            {activeScenarioName}
                          </span>
                        </>
                      )}
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <button
                        onClick={() => handleToggleScenario(skill)}
                        disabled={!activeScenario}
                        className={cn(
                          "rounded px-2 py-1 text-[13px] font-medium transition-colors outline-none",
                          enabledInScenario
                            ? "text-emerald-600 dark:text-emerald-400 hover:bg-emerald-500/10"
                            : "text-muted hover:bg-surface-hover hover:text-secondary"
                        )}
                      >
                        {enabledInScenario ? t("mySkills.enabledButton") : t("mySkills.enable")}
                      </button>
                    </div>
                  </div>
                </div>
              );
            }

            return (
              <div
                key={skill.id}
                className={cn(
                  "app-panel group flex items-center gap-3.5 rounded-xl border-transparent px-3.5 py-3 transition-all hover:border-border hover:bg-surface-hover",
                  enabledInScenario && "border-l-2 border-l-accent"
                )}
              >
                {isSynced ? (
                  <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-emerald-500" />
                ) : (
                  <Circle className="h-3.5 w-3.5 shrink-0 text-faint" />
                )}

                <h3
                  className="w-[180px] shrink-0 truncate cursor-pointer text-[14px] font-semibold text-secondary hover:text-primary"
                  onClick={() => openSkillDetailById(skill.id)}
                  title={skill.name}
                >
                  {skill.name}
                </h3>

                <p className="min-w-0 flex-1 truncate text-[13px] text-muted">
                  {skill.description || "—"}
                </p>

                <span
                  className="min-w-0 max-w-[240px] truncate font-mono text-[11px] text-faint"
                  title={sourceSummary(skill)}
                >
                  {sourceSummary(skill)}
                </span>

                {revisionSummary(skill) && (
                  <span className="shrink-0 font-mono text-[11px] text-faint">
                    {revisionSummary(skill)}
                  </span>
                )}

                <div className="flex shrink-0 items-center gap-1.5">
                  {skill.tags.map((tag) => (
                    <span
                      key={tag}
                      className="inline-flex items-center rounded-full bg-accent-bg px-1.5 py-0.5 text-[11px] font-medium text-accent-light"
                    >
                      {tag}
                    </span>
                  ))}
                </div>

                <div className="flex shrink-0 items-center gap-2.5">
                  {sourceBadge && (
                    <span className={cn("rounded-full px-2 py-0.5 text-[11px] font-medium", sourceBadge.className)}>
                      {sourceBadge.label}
                    </span>
                  )}
                  {badge && (
                    <span className={cn("rounded-full px-2 py-0.5 text-[11px] font-medium", badge.className)}>
                      {badge.label}
                    </span>
                  )}
                  <span className="inline-flex items-center gap-1 text-[13px] text-muted">
                    {sourceIcon(skill.source_type)}
                    {sourceTypeLabel(skill)}
                  </span>
                  {enabledInScenario && (
                    <span className="text-[13px] font-medium text-amber-600 dark:text-amber-400/80">
                      {activeScenarioName}
                    </span>
                  )}
                </div>

                <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
                  <button
                    onClick={() => handleToggleScenario(skill)}
                    disabled={!activeScenario}
                    className={cn(
                      "rounded px-2 py-0.5 text-[13px] font-medium transition-colors outline-none",
                      enabledInScenario
                        ? "text-emerald-600 dark:text-emerald-400 hover:bg-emerald-500/10"
                        : "text-muted hover:bg-surface-hover hover:text-secondary"
                    )}
                  >
                    {enabledInScenario ? t("mySkills.enabledButton") : t("mySkills.enable")}
                  </button>
                  <button
                    onClick={() => handleCheckUpdate(skill)}
                    disabled={checkingSkillId === skill.id}
                    className="rounded p-0.5 text-muted transition-colors hover:bg-surface-hover hover:text-secondary disabled:opacity-50"
                    title={t("mySkills.updateActions.check")}
                  >
                    <RefreshCw className={cn("h-3.5 w-3.5", checkingSkillId === skill.id && "animate-spin")} />
                  </button>
                  {canRefresh(skill) ? (
                    <button
                      onClick={() => handleRefreshSkill(skill)}
                      disabled={updatingSkillId === skill.id}
                      className="rounded p-0.5 text-accent-light transition-colors hover:bg-accent-bg disabled:opacity-50"
                      title={refreshLabel(skill)}
                    >
                      <RotateCcw className={cn("h-3.5 w-3.5", updatingSkillId === skill.id && "animate-spin")} />
                    </button>
                  ) : null}
                  <button
                    onClick={() => setDeleteTarget(skill)}
                    className="rounded p-0.5 text-faint transition-colors hover:text-red-400"
                    title={t("mySkills.delete")}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <SkillDetailPanel
        skill={selectedSkill}
        onClose={closeSkillDetail}
        syncMeta={selectedSkill ? getSyncMeta(selectedSkill) : null}
        syncing={selectedSkill ? syncingSkillId === selectedSkill.id : false}
        onSync={(mode) => selectedSkill && handleSyncAction(selectedSkill, mode)}
      />

      <ConfirmDialog
        open={deleteTarget !== null}
        message={t("mySkills.deleteConfirm", { name: deleteTarget?.name || "" })}
        onClose={() => setDeleteTarget(null)}
        onConfirm={handleDeleteManagedSkill}
      />
      <ConfirmDialog
        open={originAuditOpen}
        title={t("mySkills.sourceAuditTitle")}
        message={
          originAuditPlan
            ? t("mySkills.sourceAuditMessage")
            : t("common.loading")
        }
        details={
          originAuditPlan
            ? [
                t("mySkills.sourceAuditSummary", {
                  auto: originAuditPlan.entries.filter((entry) => entry.action === "write-custom-no-source").length,
                  review: originAuditPlan.entries.filter((entry) => entry.action === "needs-network-review").length,
                  kept: originAuditPlan.entries.filter((entry) => entry.action === "keep").length,
                }),
                originAuditPlan.root,
              ]
            : []
        }
        confirmLabel={t("mySkills.sourceAuditApply")}
        tone="warning"
        onClose={() => setOriginAuditOpen(false)}
        onConfirm={handleApplyOriginAudit}
      />
      <ConfirmDialog
        open={runtimePlacementOpen}
        title={t("mySkills.runtimePlacementTitle")}
        message={t("mySkills.runtimePlacementMessage")}
        details={
          runtimePlacementPlan
            ? [
                t("mySkills.runtimePlacementSummary", {
                  label: "Codex",
                  ...summarizePlacement(runtimePlacementPlan.codex).counts,
                }),
                ...summarizePlacement(runtimePlacementPlan.codex).samples.map(
                  (sample) => `Codex · ${sample}`
                ),
                t("mySkills.runtimePlacementSummary", {
                  label: "Global",
                  ...summarizePlacement(runtimePlacementPlan.global).counts,
                }),
                ...summarizePlacement(runtimePlacementPlan.global).samples.map(
                  (sample) => `Global · ${sample}`
                ),
              ]
            : []
        }
        confirmLabel={t("common.close")}
        tone="warning"
        onClose={() => setRuntimePlacementOpen(false)}
        onConfirm={async () => {
          setRuntimePlacementOpen(false);
        }}
      />
    </div>
  );
}
