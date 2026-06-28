# lateo

[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/ljh-sh/lateo/badge)](https://scorecard.dev/viewer/?uri=github.com/ljh-sh/lateo)

> 图像的不可见数据嵌入 —— 隐写(藏起来的*消息*)与水印(藏起来的*烙印*),共用一个引擎。

> Imperceptible data embedding for images — steganography (covert *message*) and watermarking (robust *imprint*). — [English](README.md)

## TL;DR

```bash
lateo hide    image.png  -m secret.txt   # 嵌入隐秘密文(脆弱)
lateo extract image.png                  # 取回密文
lateo mark    image.png  -i owner-id      # 嵌入鲁棒烙印
lateo verify  image.png                  # 检查烙印是否存活
lateo probe   image.png                  # 反向自检:能否被检测 / 剥离?
```

> **状态:骨架。** 上述命令已接线,但嵌入引擎尚未实现。`lateo <verb>` 目前会报告"未实现"。

## 这是什么

`lateo` 在一个 CLI 后面放了两个信息隐藏原语:

| | 隐写 steganography | 水印 watermark |
| --- | --- | --- |
| 藏的是什么 | 一段隐秘**消息** | 一个鲁棒**烙印** |
| 存在性 | **否认**(没人知道) | **宣告**(你公开声明) |
| 鲁棒性 | 脆弱(重编码即死) | 扛得住重编码 |
| 容量 | 高 | 低 |

它们是刻意分开的动词,而不是一个"藏数据"调用:两者的优化目标是对立的(不可见+容量 vs 鲁棒),所以即便共用图像 I/O 与变换管线,也需要各自的引擎。

名字来自拉丁语 ***lateo*** —— *"我隐藏着"*,是 *latent*(潜在)的词根。

## 安装

**从源码构建**(完整功能):

```bash
cargo install --git https://github.com/ljh-sh/lateo
```

预编译、cosign 签名的二进制(Linux x86_64/arm64、macOS x86_64/arm64、Windows x86_64)将随首个 release 附上。

## 许可

Apache-2.0。
