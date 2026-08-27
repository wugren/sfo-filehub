# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: trivial
- Change record: not-applicable
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - 按用户第三轮要求「使用应该基于 github action 发布之后的方式」，根
    `README.md` 的使用指引改为以 GitHub Actions 发布产物为入口；第四轮按
    「github 用户名是确定的」把 GHCR 与 Releases 用户名占位符实写为 `wugren`：
    快速开始不再从源码构建，而是（a）`docker pull ghcr.io/wugren/filehub:v<版本>`
    后
    `docker run` 直接获得服务端 + 管理后台一体环境，（b）从 GitHub Release
    下载 4 个归档（`filehub-server_<ver>_linux_x86_64.tar.gz`、CLI
    linux-x86_64 / macos-aarch64 / windows-x86_64）；
  - 第五轮按用户要求「服务端配置应该通过配置文件来控制，文档中不要包含环境
    变量的方式」：根 README 移除全部服务端环境变量配置内容——Docker 快速开始
    的 `-e FH_ADMIN_PASSWORD`、`FH_SESSION_KEY` 等环境变量清单、`FH_CONFIG`
    说明、CLI 段的 `FILEHUB_*` 清单与安全提醒中的 `FH_SESSION_KEY` 建议；
    服务端配置只通过 `filehub-server.json`（命令行参数）描述，Docker 段保留
    卷挂载并链接 [docker/README.md](docker/README.md)；
  - 保留完整使用操作：Web 后台五步路径（建项目→建版本→上传 app→下载/锁定，
    SHA-256 页面自动计算）与 CLI 工作流（`login`→`new-version`→`push`→
    `versions`→`pull`→`lock-version`，目标串 `host[:port]/project/version/name`）；
  - 新增「GitHub Actions 发布说明」小节，按 `build.yml` 实际行为描述：`v*`
    标签触发发布（4 个 Release 归档 + GHCR 镜像，tag 必须匹配 workspace
    版本，规范仓库 `wugren/sfo-filehub`），`workflow_dispatch` 仅产出临时
    artifact 不发布，CI 仅在 Linux 跑 Rust 全量测试、三平台构建 CLI；
  - 源码构建（cargo/npm）降级为文末「开发者/本地构建」小节；测试、配置/安全、
    文档索引保留。
- Handoff: README 中 13 个链接（11 个仓库相对 + 2 个 GitHub 外部链接）全部
  可解析，用户名占位符已全部清除，`FH_*`/`FILEHUB_*`/「环境变量」字面为零；
  Markdown 围栏成对闭合；Release 归档名、
  GHCR 镜像名（`ghcr.io/wugren/filehub`）、发布触发条件逐条与
  `.github/workflows/build.yml` 和 `git remote`（`github.com/wugren/sfo-filehub`）
  核对一致；`git status` 项目级改动仅 `README.md`。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-root-readme | 根 `README.md` 的使用说明以 GitHub Actions 发布产物为主：从 GitHub Release 下载 CLI/服务端归档、拉取 GHCR 镜像运行，再给 Web/CLI 操作示例；源码构建只保留为开发路径；服务端配置一律以 `filehub-server.json` 配置文件描述，README 不出现 FH_*/FH_CONFIG 等服务端环境变量配置方式 | proposal.md P-001 | README「快速开始（基于 GitHub Actions 发布产物）」「GitHub Actions 发布说明」「开发者/本地构建」三节落地：Release 4 归档表、`ghcr.io/wugren/filehub` pull/run 命令、`wugren/sfo-filehub` Releases 链接与 build.yml 声明一致；Docker 段无 `-e`/FH_* 字面，配置章节只讲配置文件 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | README 使用路径与 `.github/workflows/build.yml` 的 artifact 上传、Release 资产命名、GHCR 镜像名、tag 门禁逻辑逐条比对；CLI/Web 示例与 `cli/src/cli/args.rs`、admin-web 页面代码比对；全文对 `FH_*`/`FILEHUB_*`/环境变量作字面扫描 | 代换「把 macos-aarch64/windows 平台写错」「把手动触发写成会发布」「把版本号写死成不存在的 tag」「把源码构建留在使用主路径」「把环境变量配置方式写回 README」等反例 | 4 个归档名一字不差；手动触发仅 artifact、tag 触发才发布的描述与 workflow_dispatch / push 分支一致；源码构建明确降级为开发小节；字面扫描 0 个环境变量令牌，服务端配置只出现配置文件路径与字段 | pass |
| boundaries-and-failure-paths | README 13 个链接（相对 + 外部）、`server/src/model/config.rs` 配置字段、`docker/entrypoint.sh`/`docker/nginx.conf`、服务端归档目录结构（`server/`、`web/`）、`git remote` | 检查「链接指向不存在文件」「镜像名 owner 与实际 remote 不符」「服务端归档里没有 nginx，直接说解压即跑」「nginx 反代路径写错」「把容器直挂配置文件写成当前已支持能力」等边界 | 相对链接 11/11 存在、2 个 GitHub 外部链接与 remote `github.com/wugren/sfo-filehub` 一致；GHCR owner 实写 `wugren`；admin-web 归档需 nginx 托管如实说明；Docker 段未声称支持直挂配置文件，入口机制仅保留 docker/README.md 链接 | pass |
| regression-and-side-effects | `git status` 清单、pre-edit 基线（本轮捕获修订前 README 快照）、既有 cli/docker README 与 docs/ 内容 | 检查本轮是否改写 CI/代码/既有文档、是否误把 CI 的 artifact 命名 bug 描述成已发布事实 | 项目级改动仅 README.md；`build.yml` 等零改动；README 明确按设计意图描述发布产物并把 CI 命名不一致作为发现记录（F-1），未伪装成已修复 | pass |

## Verification

- Targeted check: 链接检查脚本（11 个 Markdown 相对链接全部存在，2 个 GitHub
  外部链接与 git remote 一致）；`rg` 扫描确认 `FH_*`/`FILEHUB_*`/「环境变量」
  在 README 中 0 命中；占位符扫描确认 `GitHub 用户名`/`owner` 类占位已清除；
  围栏闭合检查（12 行围栏标记为偶数）；逐行比对 README 归档表与
  `.github/workflows/build.yml` 第 145-153/236-249/291-305 行的产物命名与触发
  条件；`git status` 确认项目级仅 README.md
- Result: pass
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `build.yml` build job 上传 server artifact 名为 `server-binary`，build-image/release 下载名为 `filehub-server`，两侧不一致 | 当前 CI 的镜像构建/发布阶段可能取不到 server 二进制，GitHub Release 产物尚无法完整产出；README 按设计意图描述发布物，CI 修复已明确不在本任务范围（用户在确认中未选择「确认并修复 CI」），建议单独任务处理 | no |
| F-2 | low | README 为静态快照；Release 归档需真实发布一次后才存在，README 使用版本占位、owner 已实写 wugren | 发布链路或产物命名变化后 README 需同步维护；已链接 build.yml 与 docker/ci 文档降低漂移 | no |
| F-3 | low | 根 README 已按要求移除服务端环境变量内容，但 `docker/README.md` 仍按入口脚本的 FH_* 机制编写 | 两处文档呈现口径不同：根 README 只提供配置文件和 docker/README 链接；容器若需直接挂载配置文件支持，属镜像行为变更，应另开任务（本任务明确不在内） | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 第五轮修订按用户确认范围完成：根 README 移除全部服务端环境变量配置
  方式（FH_*/FH_CONFIG/FILEHUB_* 字面为零），服务端配置只通过
  `filehub-server.json` 描述，Docker 段保留卷挂载与 docker/README 链接；使用
  入口基于 GitHub Actions 发布产物，GHCR 与 Releases 用户名实写 `wugren`；
  链接 13/13 可解析，项目级改动仅 README.md；F-1/F-2/F-3 均为既有问题或文档
  口径记录，不阻塞交付。
