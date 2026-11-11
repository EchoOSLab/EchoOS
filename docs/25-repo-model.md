# 25 · 仓库模型：公开上游 + 私有下游

**状态**：2026-11-11 生效 ｜ **模式**：方案 A（Open Core，Public Upstream）
**公共上游**：`OpenEchoOS` → `git@github.com:EchoOSLab/EchoOS.git`（remote 名 `github`）

## 1. 角色

| 仓库 | 角色 | 远程 |
|---|---|---|
| **OpenEchoOS** | 公共 canonical upstream；唯一 source of truth | `github`（公开） |
| **EchoOS** | 私有 downstream；客户/商业/认证集成 | `upstream` → GitHub；内部远程（不公开） |

规则：

1. **只有一个上游**：公共仓库是公共代码的唯一来源；
2. **单向同步**：公共 → 私有是常态；私有 → 公共必须走干净的 PR；
3. **公共历史不 rewrite**：打 tag 后尤其禁止；
4. **私有代码不得直接进入公共**：所有公共提交必须可公开审计。

## 2. 目录与代码边界

| 层 | 公共 OpenEchoOS | 私有 EchoOS |
|---|---|---|
| 内核/调度/IPC/时间/PAL/codegen | ✅ | 只做集成 |
| QEMU/开发板 BSP（H750、Nucleo、QEMU） | ✅ | — |
| 客户板卡 BSP、原理图、校准 | ❌ | ✅ |
| 商业现场总线实现、安全认证证据 | 接口/抽象 | ✅ 实现与证据 |
| 客户数据、HIL 报告、签名密钥 | ❌ | ✅ |

**硬规则**：公共 workspace 不得引用私有 crate；公共仓库必须能独立 `cargo test` / `cargo xtask graph`。

## 3. 同步流程

### 3.1 公共 → 私有

```bash
# 私有仓库
git fetch upstream
git merge upstream/main        # 私有 main 集成公共发布
```

### 3.2 私有 → 公共

```bash
# 从公共上游切干净分支，禁止把私有历史带过去
git checkout -b fix/foo upstream/main
# ... 修改 ...
git push github fix/foo        # 在 GitHub 开 PR
```

合并后再同步回私有。规则：**通用修复 upstream-first**；私有 hotfix 若不能公开，必须记录原因并设定期限。

### 3.3 禁止

- 公共仓库配置私有 remote；
- `git merge` 公共/私有两条无关联历史；
- 在公共仓库里提交客户名称、密钥、未公开数据；
- 双向 merge。

## 4. 分支与发布

| 项 | 公共 OpenEchoOS | 私有 EchoOS |
|---|---|---|
| 主分支 | `main`（保护，PR + CI） | `main`（集成） |
| 功能分支 | `feat/*`、`fix/*` | `feat/*`、`customer/*` |
| 发布分支 | `release/0.x` | 按客户/版本 |
| Tag | `v0.1.0-alpha.1`、`v0.1.0` | `v0.1.0-private.1` |

- 公共版本 SemVer；PAL v0.1 是第一个 API 边界；
- 私有版本 = 公共 tag + 私有模块；
- 公共 tag 必须签名，发布说明由 Conventional Commits 生成。

## 5. CI 与质量门

**公共 CI（无 secrets）**：

```text
fmt + clippy + test + QEMU matrix + docs link check + commit lint + license check
```

**私有 CI（自托管/HIL）**：

```text
公共 CI + 客户 BSP + 硬件在环 + 证据归档
```

公共 PR 不得依赖私有 runner。

## 6. 新下游的迁移规则

私有仓库若已有独立历史，**不要**直接 `git merge --allow-unrelated-histories`。二选一：

1. 保留旧历史为 `legacy/main`，把 `main` 重置到某个公共 tag，再叠加私有提交；
2. 新建下游分支，从公共 tag 开始，旧历史只做归档。

迁移前必须备份；迁移后验证：

```bash
cargo test
cargo xtask graph
git log --oneline --graph
```

## 7. 维护节奏

- 每周：公共 issue/PR triage；私有同步；
- 每两周：公共 alpha；私有客户构建；
- 每月：公共 roadmap/RFC；私有认证复盘；
- 每季度：许可证/安全/治理复盘。

> 公共仓库是产品，私有仓库是交付；不要让私有 WIP 污染公共历史。
