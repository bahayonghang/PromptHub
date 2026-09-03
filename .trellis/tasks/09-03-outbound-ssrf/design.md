# 出站 SSRF 设计

复用 `prepare_public_url`。禁止各服务自建「只检查 scheme」的客户端。

AI 与 sync：

- `redirect(Policy::none())`
- 每 3xx：join Location → 再 `prepare_public_url`
- 默认拒绝非公网；可选 `allowPrivateNetwork` 设置，默认 false

Evaluation Bearer：仅当 redirect 后的 host+scheme 与原 URL 相同才附加。否则去掉 Authorization。

Media：下载 body 走现有 `detect_image_format`；`image/svg+xml` 失败。

IPv6：对 `2002::/16` 取后 32 bit，对 `64:ff9b::/96` 取最后 32 bit，交给 `is_public_ipv4`。
