# 25 · 仓库模型：单一开源版本

**状态**：2026-11-11 生效 ｜ **模式**：Single Open-Source Version
**公开仓库**：`EchoOS` → `git@github.com:EchoOSLab/EchoOS.git`（remote 名 `github`）

## 1. 决策

**只维护一个开源版本。**

- 不再保留私有 fork / 私有版本；
- 不再维护第二套内核；
- 所有代码、文档、测试、发布都在这一个仓库；
- 旧私有仓库已删除，历史备份保留在仓库外的 bundle 中。

这样做的理由：

1. 单人/小团队没有维护双版本的预算；
2. 双版本必然 drift，最后变成两个项目；
3. 开源版本需要成为唯一、可信、可构建的 source of truth；
4. 商业价值应来自支持、认证、集成、HIL 服务，而不是分叉内核。

## 2. 仓库与远程

| 远程名 | 地址 | 用途 |
|---|---|---|
| **`github`** | `git@github.com:EchoOSLab/EchoOS.git` | **公开 canonical upstream** |
| `origin` | 内部 `huanguo` 镜像 | 仅备份，不是第二个版本 |

- `main` 跟踪 `github/main`；
- `git push` 默认推送到 `github`；
- 内部镜像只做备份，不接受独立开发。

## 3. 分支与发布

| 项 | 规则 |
|---|---|
| 主分支 | `main`，保护，PR + CI |
| 功能分支 | `feat/*`、`fix/*`，短生命周期 |
| 发布分支 | `release/0.x` |
| Tag | `v0.1.0-alpha.1`、`v0.1.0` |
| 合并 | Squash merge，保持公共历史干净 |

- 公共历史打 tag 后**不再 rewrite**；
- 版本遵循 SemVer；
- 发布说明由 Conventional Commits 生成；
- 所有 release 必须能从公开源码复现。

## 4. 代码边界

所有进入本仓库的代码都必须是可公开审计的。

| 内容 | 是否入仓 |
|---|---|
| 内核/调度/IPC/时间/PAL/codegen | ✅ |
| QEMU/开发板 BSP | ✅ |
| 测试、文档、脚本 | ✅ |
| 客户数据、密钥、签名私钥 | ❌ |
| 客户 NDA 资料、未公开原理图 | ❌ |
| 认证机构的受限材料 | ❌（另行保管） |

如果未来出现客户定制：优先 upstream；确实不能公开的部分放在仓库**之外**的交付目录，不 fork 内核。

## 5. CI

```text
fmt + clippy + test + QEMU matrix + docs link check + commit lint + license check
```

- CI 不依赖私有 runner；
- 硬件在环/HIL 作为可选工作流，不阻塞公共构建；
- 没有真机数据时，不写“实时性通过”。

## 6. 维护节奏

| 周期 | 动作 |
|---|---|
| 每周 | issue/PR triage、CI 绿率检查 |
| 每两周 | alpha 发布 |
| 每月 | roadmap / RFC 评审 |
| 每季度 | 许可证、安全、治理复盘 |

## 7. 一句话

> **一个仓库、一个版本、一个上游；不 fork 内核，不分叉历史。**
