# 统一出站 HTTP SSRF

## Goal

所有出站 HTTP（AI、媒体、sync、evaluation 重定向）遵守仓库契约：`SSRF_BLOCKED`，每跳复查，DNS pin。

## User value

应用不能被当成访问环回、链路本地或云元数据的代理；provider 重定向不能把 API key 带到第三方。

## Confirmed facts

- AGENTS.md：AI、媒体、sync 必须过 SSRF。
- `network_safety.rs:96-161` 已用于 media 与 evaluation。
- `ai.rs:36-40,174-178` 显式豁免，且 reqwest 默认跟随重定向。
- `sync.rs` WebDAV/S3 只校验 http(s)+host。
- `evaluation.rs:742-773` 每跳复查 URL，但 Bearer 原样带到 Location。
- `media.rs` 下载信 `Content-Type`，保存路径才验 magic。
- `network_safety.rs` 未解码 6to4 `2002::/16` 与 NAT64 `64:ff9b::/96` 内嵌 IPv4。

## Requirements

- **R1** `ai.request` / `ai.stream`（若命令仍注册）使用 `prepare_public_url`，禁止自动重定向；每跳再检查。localhost / RFC1918 / 链路本地默认 `SSRF_BLOCKED`。
- **R2** WebDAV/S3 同样 pin + 禁自动重定向。允许私网的唯一入口是 settings 里默认关闭的 `allowPrivateNetwork`（名称以实现为准）；关闭时与 R1 相同拒绝。
- **R3** evaluation/media 跨 host 重定向不得转发 `Authorization`。
- **R4** 图片下载按 magic bytes 识别 JPEG/PNG/GIF/WebP；拒绝 `image/svg+xml`。
- **R5** 6to4/NAT64 内嵌私网 IPv4 视为非公网。

## Acceptance Criteria

- [x] `ai.request` 到 `http://127.0.0.1/` 或 `http://169.254.169.254/` 返回 `SSRF_BLOCKED`，无连接。
- [x] 公网 URL 3xx 到环回返回 `SSRF_BLOCKED`。
- [x] 开关关闭时，WebDAV test 到 `http://localhost` 返回 `SSRF_BLOCKED`。开关打开后同一 URL 才允许发出（仍须是 http/https）。
- [x] 伪造 `Content-Type: image/png` 的 SVG 下载失败。
- [x] 重定向到不同 host 的 evaluation 请求不含原 Bearer（测试可用录制 adapter）。
- [x] `network_safety` 单测覆盖 `2002:a00:1::` 一类内嵌 `10.0.0.1` 的地址。

## Out of scope

- 从 `invoke_handler` 删除 `ai.*`（`09-03-webview-hardening`）。本任务封住 service 传输，即使命令被重新注册也不能打私网。
- 主密码与 CSP。

## Decisions

- 2026-09-03：默认拒绝 localhost / RFC1918。Ollama 或局域网 NAS 必须打开默认关闭的显式开关。

## Evidence

`services/ai.rs:36-40,174-178`；`services/sync.rs:138-171,399-404`；`services/evaluation.rs:742-773`；`services/media.rs:529-567`；`services/network_safety.rs:53-84`。
