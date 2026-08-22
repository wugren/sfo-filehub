# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/020-github-actions-build-release.md

## Delivery Summary

- Outcome: 按确认口径新增唯一工作流 `.github/workflows/build.yml`：
  `push master` 编译+测试+构建 Docker 镜像（不推送）、`pull_request` 编译+测试、
  `push v*` 标签编译+测试+构建镜像并发布；版本 job 用 cargo metadata 解析
  workspace 版本并执行两道门控（v 标签等于版本、canonical 仓库才允许发布）；
  tag 发布 = GHCR 镜像（`ghcr.io/{owner}/filehub:v{version}`，owner 小写、只用
  GITHUB_TOKEN）+ GitHub Release 四件统一 `.tar.gz`（server+web 一份、CLI
  linux/macos/windows 三份），Release 已存在时 `--clobber` 上传，可重跑。
- Handoff: 推送 master 观察 CI 结果；推送与根 Cargo.toml workspace 版本一致的
  `v*` 标签触发发布；错误标签/非 canonical 仓库在发布前失败。hosted runner
  实跑与真实 GHCR/Release 发布留待仓库推送后验证（作为证据边界记录）。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-single-ci-rust | 唯一工作流 Rust 部分：stable + cargo test --workspace --all-targets + release 构建 server（Linux）与 CLI（三平台矩阵） | proposal.md P-001 | build.yml test-rust/build-cli job 与本地等价命令实跑通过（workspace 全测试、server/cli release 构建成功） | 交付与提案一致 | pass |
| fh-single-ci-web | build.yml admin-web 部分：npm ci + test:unit + test:integration + test:dv | proposal.md P-002 | build.yml test-web job；本地实跑 40 unit + 7 integration + dv 全部通过 | 交付与提案一致 | pass |
| fh-single-docker-image | master 构建镜像不推送；v 标签推送 GHCR，tag 为 ghcr.io/{owner}/filehub:v{version}，复用 018/019 纯运行时 Dockerfile 与最小上下文 | proposal.md P-003 | build.yml build-image job：上下文组装（真实 server 二进制 + dist + 三个 docker 运行文件，无 target/node_modules）与 Dockerfile COPY 路径核对通过；ghcr push 用 GITHUB_TOKEN | 交付与提案一致 | pass |
| fh-single-release-publish | v 标签发布：版本/仓库双门控 + 统一 .tar.gz + Actions Artifacts + GitHub Release | proposal.md P-004 | 门控 shell 模拟 6 场景通过；打包模拟得 4 件合法 tar.gz（server 归档含 server/ 与 web/ 目录）；gh release create/upload 幂等分支实现 | 交付与提案一致 | pass |
## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 逐行审查 build.yml：触发矩阵、version job 门控、job 依赖与 if 条件、并发组、权限最小化、镜像 tag、gh release 幂等 | 挑战 tag 与版本不符、非 canonical 仓库、已有 Release 重跑、artifact 下载合并、Windows tar 打包 | actionlint 1.7.12 通过；门控 6 场景模拟符合预期；重跑路径 upload --clobber；merge-multiple 下载 CLI 归档；未发现行为缺陷 | pass |
| boundaries-and-failure-paths | pull_request 分支、fork 仓库、publish=false 分支、master 只构建不推送、artifact 缺失/权限、Windows 原生二进制残留 | 挑战错误 tag 是否在发布前失败、非 canonical 是否只构建、master 是否误推 GHCR、二进制执行位是否在 artifact 往返后丢失 | 门控在发布前失败；非 canonical 与 branch/PR 均 publish=false；build-image job 因 `if: push` 不跑 PR，且 assemble 步骤 chmod +x 恢复执行位；master 不执行 docker push | pass |
| regression-and-side-effects | 对比产品源码、build-docker.sh、docker/ 运行文件、Cargo/package 锁文件与现有 CI | 搜索是否误改源码/构建脚本/镜像语义、npm ci 是否破坏 node_modules 恢复能力、.gitignore 新增行是否误伤其它路径 | 产品源码、构建脚本、docker 文件与锁文件零改动；CI 等价命令本地全绿后 node_modules 恢复可用；gitignore 仅匹配 `node_modules.bak-esbuild-stuck/` 目录 | pass |

## Verification

- Targeted check: actionlint 1.7.12 + python yaml；`cargo test --workspace
  --all-targets`；`cargo build --release -p filehub-server -p filehub-cli`；
  `npm ci` + `test:unit`/`test:integration`/`test:dv`；
  版本/仓库门控 shell 模拟；镜像上下文与四件 tar.gz 打包结构模拟。
- Result: passed
- Exception reason: hosted runner 实跑与真实 GHCR/Release 发布需仓库推送后
  执行，本环境按与 019 相同口径以静态+本地等价验证覆盖；本机 `npm ci` 首次
  触发 Windows 挂载上 esbuild/rollup 原生二进制 EIO，已按仓库既有方案绕过
  （挪开残留目录后全新 `npm ci`）并完成全部 web 测试；无法从 Linux 删除的两个
  锁定文件保留在 `admin-web/node_modules.bak-esbuild-stuck/`（已 gitignore，
  需在 Windows 侧删除）。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本环境无 GitHub hosted runner/GHCR 凭据 | 工作流的真实执行、镜像 push 与 GitHub Release 创建未在本机实跑，属环境边界；actionlint、命令等价实跑与结构模拟已覆盖，推送仓库后按完成报告记录的步骤复验 | no |
| F-2 | low | `admin-web/node_modules.bak-esbuild-stuck/@esbuild/.../esbuild.exe` 与 `@rollup/.../rollup.win32-x64-msvc.node` 在 WSL 挂载上 unlink EIO | Windows 进程锁定的两个原生文件无法从 Linux 删除；已通过挪开旧 node_modules 全新 `npm ci` 完成验证并恢复可用的 node_modules，残留目录已加入 .gitignore，在 Windows 侧删除后不影响仓库与 CI | no |
| F-3 | low | 未配置 workflow_dispatch | 用户确认是单工作流 + v 标签驱动；无手动发布入口，补发可用 tag 推送或 Actions 重跑已有 tag 运行完成，属刻意的范围收敛而非缺陷 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付完整覆盖已确认提案 P-001~P-004：唯一工作流完成编译测试、Docker
  镜像生成与 v 标签发布（GHCR + GitHub Release 四件 tar.gz），版本/仓库双门控
  落地；actionlint 与 YAML 校验、CI 等价命令本地全绿、门控与打包结构模拟全部
  通过；独立缺陷发现三分类全 pass，仅三条 low 级非阻塞项（hosted runner 实跑
  待推送仓库后验证、Windows 锁定残留目录已忽略待 Windows 侧清理、无手动发布
  入口为确认范围），不影响已确认交付结论。
