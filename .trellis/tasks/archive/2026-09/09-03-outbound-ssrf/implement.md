# 出站 SSRF — 实现清单

1. 扩展 `network_safety` 测试（6to4/NAT64、localhost）。
2. AI client 改 pin + 手跟重定向。
3. WebDAV/S3 agent 同样改。
4. evaluation 重定向去掉跨 host Bearer。
5. media 下载 magic-byte。
6. `cargo test network_safety`、`ai`、`media`、`sync` 校验、`evaluation` SSRF 测试。
