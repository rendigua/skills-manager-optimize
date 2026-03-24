# SkillsManager 规划设计草案

状态：草案
日期：2026-03-17
项目目录：`H:\Code Project\SkillsManager`

## 1. 文档目的

把当前已确认的规划内容沉淀为后续开发的基线文档，避免后续继续推进时重复做背景归因。

本草案当前聚焦三件事：

- 明确新项目的正式定位
- 固化已确认的架构分层和目录结构
- 记录当前已知的上游选择与主要缺口

尚未展开的部分会在后续迭代中继续补充，不在本草案中假装已经定案。

## 2. 背景与问题

当前本机实际运行的是一套“全局技能数据仓 + 轻量同步脚本”的组合：

- 全局技能仓：`C:\Users\Administrator\.agents\skills`
- 当前本机技能管理能力：`C:\Users\Administrator\.agents\skills\skills-manager`
- 关键同步脚本：`C:\Users\Administrator\.agents\skills\skills-manager\scripts\ensure-agent-links.js`

这套运行态已经解决了几个本机核心问题：

- 跨 IDE 的全局 skill 同步
- `Codex .system` 保留层保护
- 冲突目录归档
- `_archives` 备份防误加载

但它仍然不是一个正式产品仓，存在这些核心缺口：

- 全局技能仓虽然是 Git 仓库，但没有 `remote`，不是可追溯上游的正式项目
- 运行态目录和源码真相混在一起，后续演进容易继续变成热修补丁
- 大量 skill 缺少统一来源元数据，无法可靠判断更新来源
- 上游参考、可回馈增强、本机特化规则，目前没有稳定边界

## 3. 上游与参考项目结论

### 3.1 主基线选择

当前确认以 `xingkongliang/skills-manager` 作为正式主基线。

原因：

- 它是完整产品，而不是单个 skill 或脚本集合
- 当前形态已经覆盖统一技能库、多工具同步、项目技能、场景、标签、Git 备份、更新跟踪
- 技术栈成熟，适合作为长期维护与回馈 PR 的主线
- 相比完全自研，更符合“尽量少重复造轮子”的方向

### 3.2 其他项目的定位

`jiweiyeah/Skills-Manager`

- 作为多工具桌面管理器参考
- 值得借鉴按工具启停、路径配置、多工具适配体验
- 不作为主替代基线

`buzhangsan/skill-manager`

- 本质是 skill 搜索与安装器
- 值得借鉴多策略安装链路与技能发现能力
- 不适合作为总管理器主线

`deanpeters/Product-Manager-Skills`

- 不是 manager，但技能库治理能力强
- 值得借鉴 catalog、metadata 校验、trigger 审核、命令层组织
- 适合作为“技能库治理样板”参考源

`vercel-labs/agent-browser`

- 不是 manager 基线
- 但 `agent-browser` / `dogfood` 已是当前本机真实安装样本
- 后续可以作为“来源记录、同步、更新检查”能力的典型验证对象

## 4. 已确认的总体定位

### 4.1 项目真身

新项目应当是“正式产品仓”，而不是继续直接在运行目录上维护。

结论：

- 正式项目仓承载产品代码、规则、文档和上游同步
- `C:\Users\Administrator\.agents\skills` 继续作为运行时数据目录
- 运行时目录不再是源码真相
- 正式项目仓才是源码真相

### 4.2 维护策略

采用 `upstream-first` 策略：

- 优先基于 `xingkongliang/skills-manager` 跟进与演进
- 通用增强优先争取整理为可回馈上游的改动
- 无法上游化的本机特殊逻辑，再以本地增强层保留

## 5. 架构分层

当前已确认采用四层结构。

### 5.1 Upstream Core

职责：

- 承载 `xingkongliang/skills-manager` 原生能力
- 尽量保持与上游一致的语义和结构

典型内容：

- 技能安装、导入、扫描
- 多工具同步
- 项目技能
- 场景、标签、Git 备份
- SQLite 状态管理和桌面 UI

### 5.2 Portable Enhancements

职责：

- 放置有通用价值、优先考虑提交 PR 的增强能力

典型内容：

- 更完整的 skill 来源元数据
- 更新检查增强
- 导入来源可追踪性
- 冲突报告与同步诊断
- 备份目录防误识别等普适治理逻辑

### 5.3 Local Policy Layer

职责：

- 放置仅适用于当前本机或当前团队环境的特化规则

典型内容：

- `Codex .system` 保留层保护
- 不同 IDE 的特殊目录规则
- `_archives` / `_backups` 处理策略
- 本机同步优先级与兼容规则

### 5.4 Runtime Data Layer

职责：

- 只存放运行时数据，不承载产品逻辑

典型内容：

- `C:\Users\Administrator\.agents\skills`
- 数据库文件
- 归档备份
- 缓存、日志、导入结果

## 6. 正式项目目录结构草案

```text
skills-manager/
├─ app/                      # 上游 UI 与应用层
├─ core/                     # 可复用的技能管理核心
│  ├─ upstream/              # 上游兼容与适配
│  ├─ metadata/              # 来源元数据、更新状态、解析逻辑
│  ├─ sync/                  # 同步、冲突检测、分发
│  └─ diagnostics/           # 健康检查、报告、排障
├─ policies/                 # 本机/本团队特化规则
│  ├─ codex/
│  ├─ backup/
│  └─ tool-overrides/
├─ docs/
│  ├─ architecture/
│  ├─ upstream-notes/
│  └─ operations/
├─ scripts/                  # 开发与迁移脚本
├─ fixtures/                 # 测试样本 skill
├─ tests/
└─ references/              # 对照项目研究结论与映射表
```

配套约束：

- `app/` 尽量贴近上游原结构，降低未来同步成本
- `core/` 不写本机硬编码路径
- `policies/` 专门承接本地特化规则
- 运行态 skill 数据不进入正式项目仓
- 对参考项目的研究结论集中写入 `references/`

## 7. 当前已识别的关键缺口

### 7.1 来源元数据治理缺口

当前本机全局 skill 目录缺少统一的文件侧来源元数据，至少尚未形成稳定的：

- `origin.json`
- `evolution.json`
- 等价 manifest

这直接导致：

- 很多 skill 无法追踪来源
- 很难判断哪些可以安全更新
- `skills check` 一类检查无法稳定覆盖全部 skill

### 7.2 上游覆盖范围缺口

虽然 `xingkongliang/skills-manager` 已有较强的产品骨架，但当前已知仍未直接覆盖：

- `Codex .system` 保留层策略
- 目录型备份误识别防护这类本机规则
- 当前本机已有的轻量同步脚本行为兼容

这意味着新项目不能假设“切到上游就自动解决一切”，仍需保留增强设计。

## 8. 来源元数据模型草案

### 8.1 设计目标

来源元数据模型的目标不是只为了“显示这个 skill 来自哪里”，而是为后续几类能力提供统一事实层：

- 更新检查
- 上游比对
- 迁移入库
- 问题追溯
- 脱离数据库后的恢复能力

### 8.2 总体方案

当前建议采用“双层模型”：

- 文件侧来源元数据
- 应用侧运行状态元数据

其中：

- 文件侧元数据是可迁移、可审计、可脱库恢复的事实层
- SQLite 中的运行状态是索引、缓存和动态状态层

结论：

- 不能只依赖数据库
- 也不应把所有动态状态都塞进 skill 文件目录

### 8.3 文件侧元数据

建议在每个 skill 目录内保留一个稳定文件，暂定命名为 `origin.json`。

职责：

- 记录 skill 的静态来源信息
- 允许 skill 跨仓库、跨机器、跨工具迁移时保留出处
- 允许数据库丢失后重建来源信息
- 所有安装入口默认生成，不能依赖后补

建议最小字段：

```json
{
  "schema_version": 1,
  "source_type": "git",
  "source_kind": "verified-upstream",
  "source_ref": "https://github.com/vercel-labs/agent-browser",
  "distribution_ref": "https://skills.sh/vercel-labs/agent-browser",
  "source_subpath": "skills/agent-browser",
  "source_branch": "main",
  "source_revision": "a865dd56e0053a894a83e0569191985232000f26",
  "installed_via": "git-subpath-import",
  "upstream_project": "vercel-labs/agent-browser",
  "imported_at": "2026-03-17T15:00:00+08:00",
  "evidence_refs": [
    "https://github.com/vercel-labs/agent-browser",
    "https://skills.sh/vercel-labs/agent-browser"
  ],
  "confidence": "high",
  "resolution_method": "github-repo-confirmed",
  "notes": "Imported from repo subdirectory"
}
```

字段说明：

- `schema_version`
  - 元数据格式版本，便于后续演进
- `source_type`
  - 取值建议：`git` / `marketplace` / `local` / `manual` / `archive` / `legacy-untracked`
- `source_ref`
  - 主要来源标识，例如 Git URL、本地路径、市场来源 ID
- `source_ref_resolved`
  - 归一化后的真实来源，例如市场安装最终解析出的 GitHub 仓库
- `source_kind`
  - 来源确认状态，建议取值：`verified-upstream` / `verified-distribution` / `inferred-upstream` / `custom-no-source` / `replaced-equivalent` / `system-reserved`
- `distribution_ref`
  - 分发入口标识，例如 `skills.sh` 页面；不等于源码真源
- `source_subpath`
  - 技能位于上游仓库中的子路径
- `source_branch`
  - 导入时使用的分支
- `source_revision`
  - 导入时锁定的提交或版本
- `installed_via`
  - 安装方式，例如 `git-subpath-import`、`zip-import`、`manual-copy`
- `upstream_project`
  - 便于 UI 与规则层统一展示的人类可读项目标识
- `imported_at`
  - 首次导入时间
- `evidence_refs`
  - 用于支撑来源判断的证据链接或本地引用
- `confidence`
  - 当前来源判断的置信度，建议取值：`high` / `medium` / `low`
- `resolution_method`
  - 来源确认的方法，例如 `github-repo-confirmed`、`marketplace-only-confirmed`、`manual-inference`
- `notes`
  - 保留必要的人工说明

### 8.3.2 默认生成规则

`origin.json` 不应只依赖审计补录，而应成为安装成功后的默认产物。

来源候选解析器默认以 `skills.sh` 为主；`SkillsMP` 只在存在可用 API key 时作为可选分发来源参与搜索。
`SkillsMP` 返回的结果只用于补充分发证据和候选来源，不应直接覆盖 GitHub 真源判断。

建议规则：

1. `install_git`
- 自动写入 `source_type = git`
- `source_kind = verified-upstream`
- `source_ref` 记录用户输入
- `source_ref_resolved` 记录归一化后的 GitHub 真仓

2. `install_from_skillssh`
- 自动写入 `source_type = marketplace`
- `source_kind = verified-distribution`
- `distribution_ref` 记录市场入口
- `source_ref_resolved` 记录实际用于更新的 GitHub 真仓

3. `install_local`
- 自动写入 `source_type = local`
- 默认 `source_kind = custom-no-source`
- 后续再由 resolver 尝试升级为真实上游

4. agent 生成 skill
- 自动写入 `source_type = manual`
- 默认 `source_kind = custom-no-source`
- 可选记录 `created_by`、`creation_mode`、`derivation_summary`
- 除非明确绑定公开上游，否则不能伪装成可自动更新的公开 skill

### 8.3.1 来源优先级规则

建议将来源确认做成固定优先级，而不是人工拍脑袋：

1. 官方源码仓 / 官方 Git 仓库
2. 官方项目站点 / 官方文档
3. 分发平台条目，例如 `skills.sh`
4. 聚合站 / 二次整理页
5. 本地推断
6. 无来源自定义

这里的关键原则是：

- `GitHub` 等源码仓优先于分发平台
- 分发平台不是不能记，而是应记录为 `distribution_ref`
- 更新默认跟 `source_ref` 指向的真源，而不是跟 `distribution_ref`

### 8.4 应用侧运行状态

SQLite 继续承载动态状态，不要求全部写回文件。

适合放在应用侧数据库中的字段：

- `remote_revision`
- `last_checked_at`
- `update_status`
- `last_check_error`
- `enabled`
- `sync_targets`
- `content_hash`

这样分层后：

- skill 随目录迁移时，不会丢失来源
- 应用仍然能高效做索引和 UI 展示
- 动态检查结果不会污染 skill 本身

### 8.5 兼容旧 skill 的策略

当前本机已有大量缺少来源信息的旧 skill，因此必须支持遗留状态。

建议规则：

- 对无来源元数据的旧 skill，先标记为 `legacy-untracked`
- 不因为缺元数据而阻止系统继续工作
- UI 中应明确展示“来源未追踪”
- 后续通过迁移工具逐步补录

这样能避免一开始就因为历史包袱导致迁移无法启动。

## 9. 更新检查与上游比对策略草案

### 9.1 目标

更新检查的目标不是“尽可能提示更多更新”，而是提供可信、可解释、可执行的更新决策。

需要解决的问题：

- 哪些 skill 可以自动检查
- 哪些 skill 只能提示人工确认
- 哪些 skill 根本没有上游信息
- 哪些本机特化规则不应被上游覆盖

### 9.2 更新检查分层

建议把 skill 分成三类：

#### A. 可追踪上游类

条件：

- 有完整 `origin.json`
- `source_type` 可解析
- 上游存在明确版本或提交可比对

典型示例：

- 从 Git 仓库子目录导入的 skill
- 市场来源可追溯的 skill

处理策略：

- 自动检查上游 revision 或版本
- 给出明确的 `up-to-date` / `update-available` / `check-failed`

#### B. 半可追踪类

条件：

- 有部分来源信息
- 但上游无法稳定计算 revision，或来源是本地目录

处理策略：

- 提示“可重新导入”或“建议人工比对”
- 不给出伪精确的升级结论

#### C. 不可追踪遗留类

条件：

- 没有来源元数据
- 或来源信息明显失效

处理策略：

- 标记为 `legacy-untracked`
- 不参与自动更新结论
- UI 中明确提示先补录来源再谈升级

#### D. 自定义无来源类

条件：

- 已做过联网核验
- 仍无法确认上游
- 或明确就是本地自定义 skill

处理策略：

- `source_kind = custom-no-source`
- 不参与自动更新
- 允许后续手工绑定上游或替换为等价公开 skill

#### E. 替换等价类

条件：

- 原始来源不可恢复
- 但已决定收敛到功能等价、可维护的公开上游

处理策略：

- `source_kind = replaced-equivalent`
- 记录 `replacement_ref` 与 `replacement_reason`
- 更新逻辑以后续替代上游为准

### 9.3 比对结果模型

建议统一输出这些状态：

- `up-to-date`
- `update-available`
- `reimport-available`
- `legacy-untracked`
- `check-failed`
- `policy-blocked`

建议同时保留与更新状态正交的来源确认状态：

- `verified-upstream`
- `verified-distribution`
- `inferred-upstream`
- `custom-no-source`
- `replaced-equivalent`
- `system-reserved`

其中：

- `policy-blocked` 用于本机策略阻止直接覆盖的场景
- 例如 `Codex .system` 或本地 overlay 明确禁止上游直接覆盖

### 9.4 本机策略与更新的关系

更新能力必须服从策略层，不能因为“上游有更新”就直接覆盖本机保留层。

规则建议：

- 上游更新只作用于可管理 skill
- 本机保留层不纳入自动覆盖
- `Codex .system` 只做发现与冲突提示，不做自动替换
- 本地策略 patch 应与上游更新结果分离展示

换句话说：

- “上游可更新”不等于“本机应直接应用”

### 9.5 典型决策流

建议后续实现时采用以下判断顺序：

1. 先看是否存在 `origin.json`
2. 再看 `source_type` 是否支持自动检查
3. 再看是否命中本机策略阻断
4. 最后才给出更新结论

这样可以避免把策略问题、来源缺失问题、网络检查失败问题混成一种“更新失败”。

## 10. 迁移路径草案

### 10.1 迁移目标

迁移不是把当前运行态目录整体搬进新仓库，而是把“正式项目仓”和“运行时数据仓”彻底分离。

迁移完成后的目标状态：

- 正式项目仓承载产品代码、策略代码、文档和测试
- `C:\Users\Administrator\.agents\skills` 继续作为运行时数据目录
- 当前轻量同步脚本中的可复用逻辑被吸收到正式项目仓
- 历史 skill 逐步补齐来源信息

### 10.2 迁移阶段

建议分四个阶段推进。

#### 阶段 A：建立正式项目仓

目标：

- 基于 `xingkongliang/skills-manager` 建立正式主仓
- 配置 `origin` / `upstream`
- 让当前项目目录成为后续唯一开发入口

产物：

- 正式 Git 仓库
- 与上游可同步的基础结构
- 本地设计文档和参考研究文件

#### 阶段 B：补齐增强边界

目标：

- 把当前已确认的增强拆成“可上游化”和“本机策略层”

至少要完成：

- `origin.json` 模型落地设计
- 更新状态模型落地设计
- `Codex .system` 保护策略边界
- 备份归档规则边界

#### 阶段 C：接入运行态

目标：

- 让正式项目仓能够读取、解释、管理现有运行态目录
- 而不是直接破坏性重建当前环境

建议策略：

- 先做只读扫描
- 再做来源补录
- 最后才做同步与覆盖动作

关键原则：

- 迁移初期默认只读优先
- 避免一上来就“自动修复”导致现有环境被改坏

#### 阶段 D：收敛旧脚本

目标：

- 让 `ensure-agent-links.js` 这类运行中脚本逐步退居兼容层
- 正式由项目内的新模块接管

注意：

- 不要求第一天就删掉旧脚本
- 在新能力稳定前，旧脚本可以作为回退路径保留

### 10.3 历史 skill 的处理原则

迁移过程中，历史 skill 不应强制一次性“洗干净”。

建议分流：

- 有明确来源的：直接生成 `origin.json`
- 能大致判断来源的：标为半追踪，等待人工确认
- 完全不可判断的：标为 `legacy-untracked`

这样做的原因：

- 当前环境已经在生产使用
- 一次性强制补齐来源，容易把迁移变成人工清洗工程

### 10.4 风险控制

迁移阶段必须显式控制这几类风险：

- 错误识别系统 skill
- 错误覆盖本地特化 skill
- 把旧备份重新暴露为可扫描 skill
- 由于来源缺失做出错误升级判断

因此迁移相关动作建议遵循：

- 先扫描
- 再分类
- 再提示
- 最后才允许执行写操作

## 11. 上游同步与回馈工作流草案

### 11.1 基本策略

当前建议采用：

- `origin`：你自己的正式维护仓
- `upstream`：`xingkongliang/skills-manager`

主线原则：

- 尽量贴近上游演进
- 通用增强优先整理成 PR
- 本机策略层尽量少侵入上游核心

### 11.2 改动分类

后续所有改动建议先分到三类，再决定落点：

#### A. 通用增强

标准：

- 对多数用户都有价值
- 不依赖你本机特定目录结构
- 不要求特殊私有环境

处理：

- 优先做成上游友好的实现
- 优先考虑提交 PR

典型示例：

- 更好的来源元数据支持
- 更新检查增强
- 冲突诊断报告

#### B. 本机策略

标准：

- 强依赖你当前环境
- 对其他用户未必成立

处理：

- 保留在本地策略层
- 避免直接侵入上游核心默认行为

典型示例：

- `Codex .system` 特殊处理
- 本机归档目录策略
- 特定 IDE 路径约束

#### C. 运行时配置

标准：

- 不属于产品逻辑
- 只是当前机器或当前项目的启用项

处理：

- 放到配置或本地设置层
- 不写死在核心代码

### 11.3 上游跟进节奏

建议后续建立固定节奏，而不是想到才看：

- 周期性检查 `upstream` 更新
- 先做差异摘要
- 再判断是否需要合并
- 最后区分“直接跟进”还是“作为参考吸收”

这样可以避免两种常见问题：

- 长时间不跟上游，最后一次性漂移过大
- 看到上游更新就立刻追，导致本机策略层反复破坏

### 11.4 冲突决策原则

当上游与本地增强冲突时，建议按下面顺序判断：

1. 能否直接删除本地增强，改用上游实现
2. 能否把本地增强重写成更薄的一层适配
3. 如果不能，是否明确保留为本机策略层

不建议的做法：

- 长期在上游核心里堆私有 patch
- 让本地逻辑和上游实现交叉缠绕

## 12. 验证策略与测试样本设计草案

### 12.1 验证目标

验证不是为了“证明设计看起来合理”，而是为了尽早发现下面几类高风险问题：

- 来源识别错误
- 更新判断错误
- 策略层误覆盖
- 迁移动作误写入
- 备份或系统 skill 被误当成正常 skill

因此验证设计必须围绕“高风险路径”展开，而不是只做表层 happy path。

### 12.2 验证分层

建议采用四层验证。

#### A. 元数据解析层

目标：

- 验证 `origin.json` 的读写与兼容
- 验证缺字段、旧版本、损坏 JSON 的降级逻辑

至少覆盖：

- 完整 `git` 来源
- `marketplace` 来源
- `manual` 来源
- `legacy-untracked`
- 非法 JSON
- 不支持的 `schema_version`

#### B. 同步与策略层

目标：

- 验证不同 tool 目录下的同步行为
- 验证本机策略层不会误伤保留路径

至少覆盖：

- 正常分发
- 目标目录已存在
- `Codex .system` 命中
- `_archives` / `_backups` 被跳过
- copy/symlink 模式差异
- 冲突只报告不覆盖

#### C. 迁移与扫描层

目标：

- 验证从现有运行态目录生成来源分类结果
- 验证只读扫描不会产生副作用

至少覆盖：

- 已有来源的 skill
- 可推断来源的 skill
- 完全无来源的 skill
- 历史备份目录
- 本地 overlay 与系统 skill 并存

#### D. 端到端冒烟层

目标：

- 验证从导入、建模、同步到更新检查的一条完整链路

至少覆盖：

- 从 Git 仓库子目录导入 skill
- 写入 `origin.json`
- 扫描入库
- 同步到目标工具目录
- 执行一次更新检查
- 命中策略阻断时返回 `policy-blocked`

### 12.3 测试样本设计

建议专门维护一组固定 fixtures，而不是每次用真实全局目录直接测。

建议样本分组：

#### 技能来源样本

- `git-subpath-agent-browser`
- `git-subpath-dogfood`
- `manual-local-skill`
- `marketplace-skill`
- `legacy-untracked-skill`

#### 策略与冲突样本

- `codex-system-shadowed-skill`
- `backup-directory-skill`
- `archive-only-skill`
- `same-name-global-local-conflict`

#### 迁移样本

- `runtime-snapshot-clean`
- `runtime-snapshot-mixed-origin`
- `runtime-snapshot-with-backups`

这些样本应尽量小、可重复、可脱离真实环境执行。

### 12.4 验证原则

后续实现时建议固定这几条原则：

- 默认先做 dry-run，再做写入
- 真实用户目录只用于最终冒烟，不作为主测试输入
- 每次新增策略规则，都要补一条对应 fixture
- 每次新增来源类型，都要补解析测试和更新测试

### 12.5 最小可行验收标准

在真正开始大规模重构或接管现有运行态之前，至少应满足：

1. `origin.json` 模型有稳定 fixture 与解析测试
2. `Codex .system` 保护有回归测试
3. `_archives` / `_backups` 不会被误扫描有回归测试
4. `legacy-untracked` 不会导致迁移流程崩溃
5. 至少有一条 Git 子目录导入到更新检查的端到端冒烟链路

## 13. 当前阶段收口判断

当前设计层面已经足够支撑进入“计划编排”阶段。

原因：

- 项目定位已明确
- 主基线已明确
- 分层与目录结构已明确
- 来源元数据模型已明确
- 更新与迁移策略已明确
- 验证与测试样本方向已明确

这意味着下一阶段不再需要继续补大方向设计，而应把这些设计拆成可执行计划。

## 14. 当前阶段结论

当前阶段已经确认的不是“开始写代码”，而是以下基线：

- 新项目要有正式项目仓
- 以 `xingkongliang/skills-manager` 作为主基线
- 运行时目录与源码真相分离
- 上游能力、通用增强、本机策略、运行时数据必须分层
- 来源元数据治理是后续推进的第一优先级之一
- 验证策略与测试样本设计已达到可编排计划的程度

本文件作为当前阶段的本地设计基线，后续继续补齐未定章节。

## 15. 实施进展（2026-03-18）

当前实现与本设计保持一致，已完成这些落地项：

- `src-tauri/src/core` 已增加 `origin_metadata`、`policy_engine`、`migration_scanner`、`update_checker` 模块。
- 同步链路已接入策略阻断和 `sync_dry_run` 设置。
- 只读迁移扫描命令已暴露为 `scan_runtime_migration`。
- 前端已对齐新状态：`update-available`、`reimport-available`、`legacy-untracked`、`policy-blocked`、`check-failed`。
- 设置页已增加 `sync_dry_run` 开关，以及迁移只读/策略保护提示。
- 夹具已补齐 `tests/fixtures/skills/git-subpath-dogfood/origin.json`，用于 Git 子目录来源样本。
- `origin.json` schema 已扩展第一版来源分级字段：
  - `source_kind`
  - `distribution_ref`
  - `evidence_refs`
  - `confidence`
  - `resolution_method`
  - `replacement_ref`
  - `replacement_reason`
- `origin_metadata` 解析测试已覆盖：
  - `verified-upstream` 与 `distribution_ref` 并存
  - `custom-no-source` 解析

当前剩余工作：

- 基于这套分级规则继续补录真实运行态 skills 的 `origin.json`
- 后续再把“需要联网核验”的分支做成独立批处理器，避免误把未知上游写成自定义
- 为 `replaced-equivalent` 增加更完整的替代上游联动策略
