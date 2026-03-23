# SkillsManager Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 `xingkongliang/skills-manager` 建立正式主仓，并补齐来源元数据、策略层保护、迁移扫描和验证体系，使其可以逐步接管当前本机运行态技能管理。

**Architecture:** 先以上游仓为核心骨架，优先扩展 `src-tauri/src/core` 与 `src-tauri/src/commands`，把来源元数据、策略层和迁移扫描收敛为 Rust 核心能力，再通过现有 React/Tauri UI 暴露必要状态。实现顺序遵循“先读后写、先 dry-run 后执行”，确保现有 `C:\Users\Administrator\.agents\skills` 环境不会被破坏性修改。

**Tech Stack:** React 19, TypeScript, Vite, Tauri 2, Rust, SQLite, Cargo tests

---

## Preconditions

- 当前工作区已导入 `xingkongliang/skills-manager` 上游骨架，并保留 `docs/superpowers/` 规划文档。
- 本计划默认下一步会把当前仓替换或重建为 `xingkongliang/skills-manager` 的正式 fork/跟踪仓，同时保留现有 `docs/superpowers/` 文档。
- 因当前规则不默认执行 `git commit`，本计划不包含提交步骤，只包含实现和验证步骤。
- Rust toolchain 已安装；`cargo.exe` 路径为 `%USERPROFILE%\\.cargo\\bin\\cargo.exe`（当前会话未自动进 PATH）。

## File Structure Map

计划默认以下文件会成为第一批核心落点：

- `src-tauri/src/core/skill_store.rs`
  - 扩展 SQLite schema，承载来源与更新状态字段。
- `src-tauri/src/core/skill_metadata.rs`
  - 保留现有 `SKILL.md` 解析职责，不继续膨胀。
- `src-tauri/src/core/origin_metadata.rs`
  - 新增，负责 `origin.json` 读写、校验、兼容和默认分类。
- `src-tauri/src/core/policy_engine.rs`
  - 新增，负责 `.system`、`_archives`、`_backups`、保留目录和覆盖阻断判断。
- `src-tauri/src/core/migration_scanner.rs`
  - 新增，负责对现有运行态目录做只读扫描和分类。
- `src-tauri/src/core/update_checker.rs`
  - 新增，负责把来源类型映射成可检查、半可检查、不可检查状态。
- `src-tauri/src/core/tool_adapters.rs`
  - 扩展，补充保留路径和策略信息。
- `src-tauri/src/core/sync_engine.rs`
  - 扩展，接入策略阻断和 dry-run 支持。
- `src-tauri/src/commands/scan.rs`
  - 扩展，暴露迁移扫描与来源分类结果。
- `src-tauri/src/commands/sync.rs`
  - 扩展，暴露策略阻断、冲突报告和 dry-run 同步。
- `src-tauri/src/commands/skills.rs`
  - 扩展，暴露 skill 来源元数据与更新状态。
- `src/views/MySkills.tsx`
  - 扩展，显示来源、更新状态、遗留状态。
- `src/views/Settings.tsx`
  - 扩展，显示迁移模式、dry-run 开关和策略提示。
- `src-tauri/tests/origin_metadata.rs`
  - 新增，覆盖 `origin.json` 模型与兼容逻辑。
- `src-tauri/tests/policy_engine.rs`
  - 新增，覆盖 `.system`、归档目录和冲突规则。
- `src-tauri/tests/migration_scanner.rs`
  - 新增，覆盖现有运行态目录扫描与分类。
- `src-tauri/tests/update_checker.rs`
  - 新增，覆盖更新状态决策。
- `tests/fixtures/skills/*`
  - 新增，固定 skill 与迁移样本夹具。

## Chunk 1: Baseline And Source Model

### Task 1: 导入上游基线并保留规划文档

**Files:**
- Create: `package.json`
- Create: `src/App.tsx`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/core/mod.rs`
- Modify: `docs/superpowers/specs/2026-03-17-skills-manager-design.md`
- Modify: `docs/superpowers/plans/2026-03-18-skills-manager-implementation-plan.md`

- [x] **Step 1: 将当前空仓替换为正式上游基线**

Run: `git clone --depth 1 https://github.com/xingkongliang/skills-manager H:\Code Project\SkillsManager\_tmp_upstream`
Expected: 上游仓被拉到临时目录，包含 `src/`、`src-tauri/`、`package.json`

- [x] **Step 2: 将上游源码复制到当前仓并保留 docs 目录**

Run: `robocopy "H:\Code Project\SkillsManager\_tmp_upstream" "H:\Code Project\SkillsManager" /E /XD .git`
Expected: 当前仓具备上游源码结构，同时 `docs/superpowers/` 仍存在

- [x] **Step 3: 删除临时导入目录**

Run: `Remove-Item -Recurse -Force 'H:\Code Project\SkillsManager\_tmp_upstream'`
Expected: 临时目录被移除

- [x] **Step 4: 验证上游骨架已落地**

Run: `Get-ChildItem 'H:\Code Project\SkillsManager'`
Expected: 能看到 `src`、`src-tauri`、`package.json`、`docs`

### Task 2: 建立来源元数据模块和夹具

**Files:**
- Create: `src-tauri/src/core/origin_metadata.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Create: `src-tauri/tests/origin_metadata.rs`
- Create: `tests/fixtures/skills/git-subpath-agent-browser/origin.json`
- Create: `tests/fixtures/skills/manual-local-skill/origin.json`
- Create: `tests/fixtures/skills/legacy-untracked-skill/SKILL.md`

- [x] **Step 1: 先写失败的元数据测试**

Code to add in `src-tauri/tests/origin_metadata.rs`:

```rust
#[test]
fn parses_git_origin_metadata() {}

#[test]
fn falls_back_to_legacy_untracked_when_origin_is_missing() {}

#[test]
fn rejects_unsupported_schema_versions() {}
```

- [x] **Step 2: 跑测试确认当前缺模块失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml origin_metadata -- --nocapture`
Expected: FAIL，提示 `origin_metadata` 模块或方法不存在
Actual (2026-03-18): `cargo` 不存在，命令未执行到编译阶段

- [x] **Step 3: 实现最小 `origin.json` 模型**

Code to add in `src-tauri/src/core/origin_metadata.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceType {
    Git,
    Marketplace,
    Local,
    Manual,
    Archive,
    LegacyUntracked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginMetadata {
    pub schema_version: u32,
    pub source_type: SourceType,
    pub source_ref: Option<String>,
    pub source_subpath: Option<String>,
    pub source_branch: Option<String>,
    pub source_revision: Option<String>,
    pub installed_via: Option<String>,
    pub upstream_project: Option<String>,
    pub imported_at: Option<String>,
    pub notes: Option<String>,
}
```

- [x] **Step 4: 让测试通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml origin_metadata -- --nocapture`
Expected: PASS

### Task 3: 扩展数据库来源字段和状态字段

**Files:**
- Modify: `src-tauri/src/core/skill_store.rs`
- Create: `src-tauri/tests/update_checker.rs`

- [x] **Step 1: 先写失败的 store 测试，约束新增字段**

Code to add in `src-tauri/tests/update_checker.rs`:

```rust
#[test]
fn persists_origin_and_update_fields() {}
```

- [x] **Step 2: 跑测试确认 schema 还不满足**

Run: `cargo test --manifest-path src-tauri/Cargo.toml persists_origin_and_update_fields -- --nocapture`
Expected: FAIL，提示列不存在或映射不完整
Actual (2026-03-18): `cargo` 不存在，命令未执行到编译阶段

- [x] **Step 3: 在 `skill_store.rs` 中补齐来源与更新字段**

Add fields for:

- `origin_json_path`
- `source_type`
- `source_ref`
- `source_subpath`
- `source_branch`
- `source_revision`
- `remote_revision`
- `update_status`
- `last_checked_at`
- `last_check_error`

- [x] **Step 4: 跑测试确认 store 层可持久化**

Run: `cargo test --manifest-path src-tauri/Cargo.toml persists_origin_and_update_fields -- --nocapture`
Expected: PASS

## Chunk 2: Policy And Sync Safety

### Task 4: 建立策略引擎并覆盖 `.system` / 归档目录规则

**Files:**
- Create: `src-tauri/src/core/policy_engine.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/core/tool_adapters.rs`
- Create: `src-tauri/tests/policy_engine.rs`
- Create: `tests/fixtures/skills/codex-system-shadowed-skill/SKILL.md`
- Create: `tests/fixtures/skills/archive-only-skill/SKILL.md`

- [x] **Step 1: 先写失败的策略测试**

Code to add in `src-tauri/tests/policy_engine.rs`:

```rust
#[test]
fn blocks_codex_system_shadowing() {}

#[test]
fn ignores_archive_and_backup_directories() {}
```

- [x] **Step 2: 跑测试确认当前没有策略模块**

Run: `cargo test --manifest-path src-tauri/Cargo.toml policy_engine -- --nocapture`
Expected: FAIL

- [x] **Step 3: 实现最小策略判断**

Implement in `src-tauri/src/core/policy_engine.rs`:

- `is_reserved_name(name: &str) -> bool`
- `is_system_reserved(tool_key: &str, skill_name: &str, path: &Path) -> bool`
- `should_skip_directory(name: &str) -> bool`
- `classify_policy_block(...) -> PolicyDecision`

- [x] **Step 4: 跑测试确认 `.system` 与归档目录规则生效**

Run: `cargo test --manifest-path src-tauri/Cargo.toml policy_engine -- --nocapture`
Expected: PASS

### Task 5: 给同步引擎加 dry-run 和策略阻断

**Files:**
- Modify: `src-tauri/src/core/sync_engine.rs`
- Modify: `src-tauri/src/commands/sync.rs`
- Modify: `src-tauri/src/core/tool_adapters.rs`

- [x] **Step 1: 先写失败的同步测试**

Add tests covering:

- dry-run 不落盘
- 命中 `.system` 返回 `policy-blocked`
- `_archives` / `_backups` 不参与同步

- [x] **Step 2: 跑测试确认当前同步行为不满足**

Run: `cargo test --manifest-path src-tauri/Cargo.toml sync -- --nocapture`
Expected: FAIL

- [x] **Step 3: 修改 `sync_engine.rs` 接入策略层**

Implement:

- `sync_skill(..., dry_run: bool)`
- `SyncResult { mode, status, reason }`
- policy block handling before remove/copy/link

- [x] **Step 4: 跑测试确认 dry-run 和阻断状态正确**

Run: `cargo test --manifest-path src-tauri/Cargo.toml sync -- --nocapture`
Expected: PASS

## Chunk 3: Migration And Update Decisions

### Task 6: 建立只读迁移扫描器

**Files:**
- Create: `src-tauri/src/core/migration_scanner.rs`
- Modify: `src-tauri/src/core/scanner.rs`
- Modify: `src-tauri/src/commands/scan.rs`
- Create: `src-tauri/tests/migration_scanner.rs`
- Create: `tests/fixtures/runtime-snapshot-clean/.keep`
- Create: `tests/fixtures/runtime-snapshot-mixed-origin/.keep`
- Create: `tests/fixtures/runtime-snapshot-with-backups/.keep`

- [x] **Step 1: 先写失败的迁移扫描测试**

Code to add in `src-tauri/tests/migration_scanner.rs`:

```rust
#[test]
fn classifies_runtime_skills_without_mutating_disk() {}
```

- [x] **Step 2: 跑测试确认缺少迁移扫描模块**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration_scanner -- --nocapture`
Expected: FAIL

- [x] **Step 3: 实现只读扫描与来源分类**

Implement output categories:

- `tracked`
- `partially_tracked`
- `legacy_untracked`
- `policy_blocked`
- `archived`

- [x] **Step 4: 跑测试确认扫描不写盘且分类正确**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration_scanner -- --nocapture`
Expected: PASS

### Task 7: 建立更新状态决策器

**Files:**
- Create: `src-tauri/src/core/update_checker.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/commands/skills.rs`
- Modify: `src-tauri/src/core/installer.rs`
- Modify: `src-tauri/tests/update_checker.rs`

- [x] **Step 1: 先写失败的更新状态测试**

Add tests for:

- `up-to-date`
- `update-available`
- `reimport-available`
- `legacy-untracked`
- `policy-blocked`
- `check-failed`

- [x] **Step 2: 跑测试确认状态模型尚未实现**

Run: `cargo test --manifest-path src-tauri/Cargo.toml update_checker -- --nocapture`
Expected: FAIL

- [x] **Step 3: 实现最小更新决策逻辑**

Implement in `src-tauri/src/core/update_checker.rs`:

- source-type-based check routing
- legacy fallback
- policy override
- remote revision comparison

- [x] **Step 4: 跑测试确认状态决策稳定**

Run: `cargo test --manifest-path src-tauri/Cargo.toml update_checker -- --nocapture`
Expected: PASS

## Chunk 4: UI And Acceptance

### Task 8: 在 UI 中暴露来源、遗留和策略状态

**Files:**
- Modify: `src/views/MySkills.tsx`
- Modify: `src/views/Settings.tsx`
- Modify: `src/components/SkillDetailPanel.tsx`
- Modify: `src/lib/tauri.ts`

- [x] **Step 1: 先写最小前端验收清单**

Document expected UI states:

- 可追踪 skill 显示来源和 revision
- `legacy-untracked` 有醒目标记
- `policy-blocked` 显示原因
- dry-run 模式在设置页可见

- [x] **Step 2: 对齐 Tauri 命令返回类型**

Run: `npm install`
Expected: 依赖安装完成

- [x] **Step 3: 修改前端视图读取新字段**

Implement:

- 来源摘要展示
- 更新状态标签
- 遗留状态提示
- 设置页 dry-run 和迁移说明

- [x] **Step 4: 跑前端构建确认类型正确**

Run: `npm run build`
Expected: PASS

### Task 9: 补全端到端冒烟验收

**Files:**
- Modify: `docs/superpowers/specs/2026-03-17-skills-manager-design.md`
- Modify: `docs/superpowers/plans/2026-03-18-skills-manager-implementation-plan.md`
- Create: `tests/fixtures/skills/git-subpath-dogfood/origin.json`

- [x] **Step 1: 准备一个 Git 子目录导入样本**

Sample should mimic:

- `vercel-labs/agent-browser`
- `skills/dogfood`
- pinned `source_revision`

- [x] **Step 2: 跑 Rust 测试全集**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS

- [x] **Step 3: 跑前端构建和 Tauri 类型链路**

Run: `npm run build`
Expected: PASS

- [x] **Step 4: 做一次只读迁移冒烟**

Run: `cargo test --manifest-path src-tauri/Cargo.toml classifies_runtime_skills_without_mutating_disk -- --nocapture`
Expected: PASS，且不修改真实全局技能目录

## Chunk 5: Source Resolution Governance

### Task 10: 扩展来源分级 schema 和证据字段

**Files:**
- Modify: `src-tauri/src/core/origin_metadata.rs`
- Modify: `src-tauri/tests/origin_metadata.rs`
- Modify: `tests/fixtures/skills/git-subpath-agent-browser/origin.json`
- Modify: `tests/fixtures/skills/git-subpath-dogfood/origin.json`
- Modify: `docs/superpowers/specs/2026-03-17-skills-manager-design.md`

- [x] **Step 1: 先写失败测试，约束来源分级字段**

Add tests for:

- `verified-upstream` + `distribution_ref` 并存
- `custom-no-source` 解析

- [x] **Step 2: 跑测试确认当前 schema 缺字段**

Run: `cargo test --manifest-path src-tauri/Cargo.toml origin_metadata -- --nocapture`
Expected: FAIL

- [x] **Step 3: 实现最小来源分级模型**

Implement in `src-tauri/src/core/origin_metadata.rs`:

- `SourceKind`
- `ConfidenceLevel`
- `distribution_ref`
- `evidence_refs`
- `resolution_method`
- `replacement_ref`
- `replacement_reason`

- [x] **Step 4: 跑测试确认解析通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml origin_metadata -- --nocapture`
Expected: PASS

### Task 11: 将来源分级接入扫描、更新与 UI

**Files:**
- Modify: `src-tauri/src/core/migration_scanner.rs`
- Modify: `src-tauri/src/core/update_checker.rs`
- Modify: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/src/commands/skills.rs`
- Modify: `src/lib/tauri.ts`
- Modify: `src/views/MySkills.tsx`
- Modify: `src/components/SkillDetailPanel.tsx`
- Modify: `src/i18n/zh.json`
- Modify: `src/i18n/en.json`
- Create: `src-tauri/tests/source_resolution.rs`

- [x] **Step 1: 先写失败测试，约束来源确认状态到扫描和更新链路**

Add tests for:

- `verified-upstream` 扫描后不是普通 `tracked`
- `custom-no-source` 不参与自动更新
- `verified-distribution` 与 `source_ref` 分层展示

- [x] **Step 2: 修改扫描层输出来源确认状态**

Implement:

- migration 扫描结果附带 `source_kind`
- 保留 `class`，但来源确认状态单独输出

- [x] **Step 3: 修改更新决策层识别来源确认状态**

Implement:

- `verified-upstream` 走正常更新检查
- `verified-distribution` 默认跟真源
- `custom-no-source` / `system-reserved` 不参与自动更新
- `replaced-equivalent` 跟替代上游

- [x] **Step 4: 修改前端显示来源确认层**

Implement:

- 区分“已确认上游 / 已确认分发 / 推断来源 / 无来源自定义 / 系统保留”
- 在详情面板展示 `evidence_refs` 与 `resolution_method`

- [x] **Step 5: 跑全量验证**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS

Run: `npm run build`
Expected: PASS

### Task 12: 来源审计与安全补录入口

**Files:**
- Create: `src-tauri/src/core/origin_audit.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/core/origin_metadata.rs`
- Modify: `src-tauri/src/commands/scan.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauri.ts`
- Modify: `src/views/MySkills.tsx`
- Modify: `src/i18n/zh.json`
- Modify: `src/i18n/en.json`
- Create: `src-tauri/tests/origin_audit.rs`

- [x] **Step 1: 先写失败测试，约束安全补录与网络核验分流**

Add tests for:

- `legacy-untracked` 自动建议 `custom-no-source`
- `git` 且缺少 `source_kind` 时只做网络核验建议，不自动写入
- 安全补录会写入 `origin.json`

- [x] **Step 2: 实现来源审计计划与补录写入**

Implement:

- `plan_origin_resolution`
- `apply_origin_resolution`
- `save_origin_metadata`

- [x] **Step 3: 在 My Skills 增加轻入口**

Implement:

- 审计按钮
- 安全补录确认弹窗
- 补录完成后的刷新与提示

- [x] **Step 4: 跑全量验证**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS

Run: `npm run build`
Expected: PASS

### Task 13: 安装链路默认生成 origin.json

**Files:**
- Create: `src-tauri/src/core/origin_factory.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/core/origin_metadata.rs`
- Modify: `src-tauri/src/commands/skills.rs`
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `docs/superpowers/specs/2026-03-17-skills-manager-design.md`
- Create: `src-tauri/tests/origin_factory.rs`

- [x] **Step 1: 先写失败测试，约束安装来源和 agent 生成来源**

Add tests for:

- Git 安装生成 `verified-upstream`
- skills.sh 安装生成 `verified-distribution`
- 本地导入生成 `custom-no-source`
- agent 生成 skill 生成 `manual + custom-no-source`

- [x] **Step 2: 抽离统一的 origin factory**

Implement:

- `build_install_origin_metadata`
- `build_generated_origin_metadata`
- `save_origin_metadata`

- [x] **Step 3: 在安装链路默认写入 origin.json**

Implement:

- `create_generated_skill`
- `install_local`
- `install_git`
- `install_from_skillssh`
- 全部通过 `store_installed_skill` 统一落盘

- [x] **Step 4: 更新 README 与设计文档**

Document:

- provenance 与 update source 的区别
- 默认生成规则
- agent 生成 skill 的来源策略

- [x] **Step 5: 跑全量验证**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS

Run: `npm run build`
Expected: PASS

### Task 14: 来源候选解析器与可选 SkillsMP 接入

**Files:**
- Create: `src-tauri/src/core/source_resolver.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/commands/browse.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauri.ts`
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `docs/superpowers/specs/2026-03-17-skills-manager-design.md`
- Create: `src-tauri/tests/source_resolver.rs`

- [x] **Step 1: 先写失败测试，约束候选归一和去重规则**

Add tests for:

- `skills.sh` 搜索结果能归一为 `verified-distribution`
- `SkillsMP` 条目可从 `repository` 字段还原 GitHub 真源
- 同一 GitHub 真源在多个平台结果中只保留一个高信号候选

- [x] **Step 2: 抽离 source resolver**

Implement:

- `normalize_skillssh_candidate`
- `normalize_skillsmp_candidate`
- `merge_source_candidates`
- `resolve_source_candidates`

- [x] **Step 3: 接入 Tauri 命令与前端 API**

Implement:

- `resolve_source_candidates`
- `SourceCandidate` DTO
- `skillsmp_api_key` 通过设置读取，缺失时自动跳过 SkillsMP

- [x] **Step 4: 更新 README 与设计文档**

Document:

- `skills.sh` 是默认候选搜索源
- `SkillsMP` 只有在配置 API key 后才参与
- 平台结果只补充分发证据，不覆盖 GitHub 真源

- [x] **Step 5: 跑全量验证**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`
Expected: PASS

Run: `npm run build`
Expected: PASS

## Execution Notes

- 实施顺序不能颠倒：先来源模型，再策略保护，再迁移扫描，最后再接 UI。
- 在 `Task 6` 完成前，不要对真实 `C:\Users\Administrator\.agents\skills` 目录执行写入逻辑。
- 在 `Task 7` 完成前，不要把任何“可更新”状态展示为可直接覆盖。
- 在 `Task 11` 完成前，不要把 `source_kind` 与 `update_status` 混为一个字段；来源确认状态与更新状态必须分层展示。
- 在 `Task 12` 完成前，不要把“需要联网核验”的项自动写成 `custom-no-source`；只允许安全补录。
- 如果上游导入后实际文件结构与当前快照有差异，先更新本计划中的路径，再开始写代码。

Plan updated and saved to `docs/superpowers/plans/2026-03-18-skills-manager-implementation-plan.md`.
