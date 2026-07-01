# lateo 实战手册 —— 真实可用案例

lateo 提供四类操作:

- `hide` / `extract` —— 隐写:把载荷藏进图像,密钥决定**藏哪里**
- `mark` / `verify` —— 水印:归属烙印(两种模式:**fragile** = 篡改检测,**robust** = 扛 JPEG 重编码)
- `probe` —— 反向自检:这张图里是否被藏了东西?

下面每个 recipe 都是一个**真实、具体的用例**,配可复制粘贴的命令、预期输出,以及一段诚实的"它能/不能证明什么"。没有造作的 demo。

> **准备。** `lateo` 需要在 `$PATH` 里。`cargo install --git https://github.com/ljh-sh/lateo`(源码),或从 [v0.1.0 release](https://github.com/ljh-sh/lateo/releases/tag/v0.1.0) 下载 cosign 签名的预编译包,把 `lateo` 二进制放进 `$PATH`。使用 `--passphrase` 的 recipe 额外需要 `--features encryption` 构建。

---

## 1. 证明这张照片是你的 —— robust 水印扛 JPEG 重编码

**场景。** 你是摄影师。往作品集或社交媒体发了一张 JPEG。后来别人(或者平台)重新上传了一遍(平台会再压一次 JPEG,这会毁掉任何 LSB 隐写和任何 fragile 水印)。你需要证明它还是你的。**robust** 水印专为扛这种场景设计 —— 它调制 8×8 块的中频 DCT 系数,能在常规的 quality ≥ 50 的 JPEG 重编码下活下来。

```bash
# 你,发布时:
$ lateo mark --mode robust -i cover.jpg \
    --id "alice@example.com" -k "$YOUR_KEY" -o marked.jpg
lateo: marked (Robust) id=alice@example.com -> marked.jpg
```

```bash
# 你,图片在外面转了一圈后(被重新下载、被重 JPEG、轻微裁切):
$ lateo verify --mode robust -i marked.jpg \
    --id "alice@example.com" -k "$YOUR_KEY"
lateo: robust watermark — 98% match (present)
```

实测(256×256 cover):JPEG q70 后 ~99% match,q50 后 ~98% match。低于 q50 匹配率会下降(`verify` 会打印具体百分比,你可以自己定阈值)。

> **它能/不能证明什么。** 验证为正,强证据说明这张图经过你的标记步骤。它**不能**证明**唯一**作者(任何你共享过密钥的人都能重打水印),也扛不住一个同时拥有原图 + 自由度的攻击者做差分。强声明请配合外部 provenance(C2PA、EXIF)。

---

## 2. 侦测"任何像素被改过" —— fragile 水印

**场景。** 你给一张截图、合同扫描、或电子证据签了名。后来你想知道:在你签名之后,**有没有任何一个像素被改过**?**fragile** 水印的精确性质就是如此 —— 任何最小改动都会让水印失效。

```bash
# 你,签名时:
$ lateo mark -i original.png \
    --id "case-2026-001" -k "$YOUR_KEY" -o signed.png
lateo: marked (Fragile) id=case-2026-001 -> signed.png
```

```bash
# 你,收到 / 审计时:
$ lateo verify -i signed.png -k "$YOUR_KEY"
lateo: fragile watermark present — id: case-2026-001
```

```bash
# 任何人(甚至你自己)改了一个像素:
$ magick signed.png -evaluate add 1% tampered.png    # 任何碰像素的工具
$ lateo verify -i tampered.png -k "$YOUR_KEY"
lateo: no fragile watermark found (wrong key, not marked, or tampered)
exit 1
```

> **它能/不能证明什么。** 水印**消失**说明字节自签名后被改过。但它**不能**证明**恶意**:无辜操作(换个编码器重存、换色彩配置、裁切后 padding)同样会破坏水印。把 fragile 水印当作**完整性证据**,不是欺诈证据。要"是不是*具体那个人*改的",需要上面再加一层不可否认签名。

---

## 3. 在图像里发送秘密消息

**场景。** 你要给接收方发一段短文本(或一个小文件),藏在封面图里,让随便看图的人只看到一张普通照片。接收方需要**密钥**才能提取;可选地,你还想要一个**口令**,这样即使有密钥也读不出原文。

### 3a. 明文隐写 —— 密钥决定"藏哪",不决定"藏什么"

```bash
# sender
$ lateo hide -i photo.png -m "meet at the bridge at midnight" \
    -k shared-key -o secret.png
lateo: hid 27 bytes (capacity 196608 bits) -> secret.png

# recipient
$ lateo extract -i secret.png -k shared-key
meet at the bridge at midnight
```

密钥驱动一个 Fisher–Yates 洗牌,决定**哪些**像素的 LSB 承载载荷位。没密钥就定位不到 envelope —— 所以从技术上,消息是"藏起来"的(洗牌 + LSB);但**内容**上,一旦提取出来就是**明文**。要内容保密,请看 3b。

### 3b. 带口令 —— 内容层加密(AEAD)

需要 `--features encryption` 构建。

```bash
# sender
$ lateo hide -i photo.png -m "the eagle flies at midnight" \
    -k shared-key --passphrase "correct horse battery staple" \
    -o secret.png
lateo: encrypted-hid 27 bytes (capacity 196608 bits) -> secret.png

# recipient (没口令 → 清晰、可操作的错误)
$ lateo extract -i secret.png -k shared-key
lateo: this envelope is encrypted; pass --passphrase to decrypt
exit 1

# recipient (有口令)
$ lateo extract -i secret.png -k shared-key \
    --passphrase "correct horse battery staple"
the eagle flies at midnight
```

底层:chacha20poly1305 AEAD,密钥用 argon2id(7 MiB、t=3)从口令派生,salt 和 nonce 每个 envelope 随机生成。口令错 → AEAD tag 校验失败 → 不会泄露出错的明文。加密 envelope 用独立 magic(`LATEOSGE`),所以明文 `extract` 加密图会干净地报"需要 --passphrase",而不是返回乱码。

### 3c. 藏二进制文件

```bash
# sender
$ lateo hide -i cover.png --message-file secret.bin -k key -o stego.png
lateo: hid 1024 bytes (capacity 196608 bits) -> stego.png

# recipient
$ lateo extract -i stego.png -k key > recovered.bin
```

> **它能/不能证明什么。** `hide` 让载荷对随便看的人**不可读**。它**不能**让 envelope **不可检测** —— 见 recipe 4。`--passphrase` 给了内容保密,但 envelope 的**存在**仍然会被 recipe 4 那种检测发现。如果你需要保密 *又* 不可归因,单靠隐写工具做不到 —— 配合一个私密通道(Signal、Tor 之类)。

---

## 4. "这张图里藏了消息吗?" —— 反向自检

**场景。** 你收到一张可疑的图(来路不明的发信人、奇怪的网站下载的等),想知道里面有没有人用 lateo 藏了东西。`probe` 跑经典 **chi-square** LSB 隐写分析(Westfeld–Pfitzmann 1999 风格),按通道(R/G/B)出数,再给一个启发式判定。还会把选中的位平面写出一张黑白 PNG 供肉眼检查。

```bash
$ lateo probe -i suspect.png
lateo: probe R — χ²=87.4,  LSB-equalised pair fraction=0.42
lateo: probe G — χ²=112.1, LSB-equalised pair fraction=0.45
lateo: probe B — χ²=95.8,  LSB-equalised pair fraction=0.44
lateo: probe verdict — probably natural
```

在"光滑"封面(渐变、天空、平面区域)上,如果有 lateo steg 载荷,**χ²** 会下降、**LSB equalised 对比例**会上升 —— 判定翻成 `likely stego (LSB equalised)`。

**肉眼检查** —— 默认把 LSB 平面(plane 0)写成图旁边一张黑白 PNG:

```bash
$ lateo probe -i suspect.png                      # 默认写 suspect.probe.plane0.png
$ lateo probe -i suspect.png --plane 1            # 看 bit-plane 1
$ lateo probe -i suspect.png --out bitplane.png    # 显式输出路径
```

如果 LSB 平面看上去是"随机噪声/杂乱",提示有嵌入;如果 LSB 平面**和**高位图内容有视觉上的对应,符合自然图。

> **它能/不能证明什么。** `probe` 是**启发式**。小载荷 + 高噪封面(纹理照片、plasma、已经被 JPEG 重压过的图)天然在雷达之下 —— equalisation 效果被封面自身的噪声盖住了。"probably natural" **不是**清白证明;"likely stego" 是让你**再看看**,不是定罪。数学在 `src/probe.rs`;判定阈值是三通道平均 equalised 对比例(默认 0.65)。

---

## 5. 挑一张最好的隐写载体 —— `lateo scout`

**场景。** 你有一个照片文件夹,想发一条秘密消息。**该用哪张?** 不同图的容量不同、基线 equalised 也不同(见 recipe 4)。`scout` 打印两者,给一个基于规则的判定,你可以比较候选,挑一张 (a) 容量够装你的载荷、(b) 基线给嵌入器留了最多"余地"来躲 chi-square 检测的图。

```bash
$ lateo scout -i candidates/sky.jpg
lateo: scout — capacity: 62208 bytes
lateo: scout R — χ²=87.4,  LSB-equalised pair fraction=0.42
lateo: scout G — χ²=112.1, LSB-equalised pair fraction=0.45
lateo: scout B — χ²=95.8,  LSB-equalised pair fraction=0.44
lateo: scout verdict — good carrier: enough capacity and a baseline that leaves detection headroom
```

在候选上跑一遍,**挑容量最大、且平均 equalised pair fraction 最低的那张**。

> **它能/不能证明什么。** scout 给你的是**启发式**排序,不是保证。它打印的数字是**未修改**封面的基线;嵌入会改变它们,而 `probe`(recipe 4)测的就是这个改变。"good carrier" 意思是"这张图大、基线还没到检测阈值"——**不是**说嵌入就不可见。要真正的反检测,需要自适应嵌入(J-UNIWARD、HUGO 之类),lateo 没实现。

## 接下来

- **载体质量 / 反取证** 特性(自适应嵌入、RS-analysis)——放在自己的 cargo feature 后,默认 build 保持零成本。
