# SoftPAL 逆向资料总览

Confirmed decisions and reuse policy: [English ADR index](adr/README.md).
Read accepted scoped records before repeating native analysis. The
[standing-art correction](adr/0002-preserve-native-sprite-size.md) supersedes
earlier automatic height fitting and full-rectangle projection assumptions.

Latest per-export audit: [PAL behavior inventory](contracts/README.md), with
exact names, addresses, binary hashes, ABI status and behavior records for the
Koikake, reference and Totsulover release DLLs. Domain reports:
[core](pal-core-behavior.md), [graphics](pal-graphics-behavior.md),
[audio/video](movie-audio-contract.md), [text/animation](text-animation-contracts.md),
and [input/buttons](input-buttons-contract.md). The generated inventory separates
export review from downstream analysis and portable implementation parity.

Current acceptance checklist: [port-status.md](port-status.md). Full engine
compatibility and platform acceptance are still incomplete.

Current runtime corrections and remaining gaps: [port-corrections.md](port-corrections.md). The earlier round summaries and test counts below are historical baselines. The effective signature catalog is regenerated from the current registry.

Latest implementation: [native action banks and save version 11](action-scheduler.md),
[PalEffectEx scene transitions](scene-transitions.md), and
[VM dispatch and raster FX](fx-vm-implementation.md).
Per-build notes: [Iroha native profile](iroha-native-profile.md) and the
[`mai` translation patch](mai-translation-patch.md) it can carry.
Native primary aliases and wait operands are corrected; raster lifecycle now
reaches CPU, wgpu, and Switch deko3d drawing. The [Vita follow-up](vita-precompiled-rendering.md)
adds ordinary GXM drawing with precompiled programs and no shader-compiler
dependency. Radial/bicubic dispatch and Vita PAL effect shaders remain incomplete.
The [earlier audit](vm-pal-shader-audit.md) is retained
as a pre-fix snapshot with separate native build identities.

更新：2026-10-23。本目录汇总目前已恢复的引擎结构、调用约定、地址证据、移植对应关系及剩余缺口。
后续修复见 [第二轮 Rust 修复](five-minute-rust-fixes.md) 和 [接口补齐记录](interface-additions.md)。下文首轮验证记录保留作基线。
这是已有成果的完整资料入口，不是“SoftPAL 全版本、全部行为已经逆向完成”的声明。
本轮在用户要求的 5 分钟整理窗口内，由多个 subagent 并行写作，主代理合并交叉核验。

## 证据与范围

目标为本机 Koikake（恋×シンアイ彼女）对应的 Game.exe / PAL.dll / PAL_d.dll。
[二进制指纹](evidence/manifest.json) 固定本次地址的适用版本。游戏目录只读，没有解包或写入游戏目录。
IDA 9.0 在复制的数据库上导出证据，没有修改原 IDB。源码和本地二进制的对应版本不能混为一谈：
FHD 多语言版本的资源/字体/文本行为有独立代码支持，但本文地址不是所有发行版通用的偏移。

证据强度区分：

1. **已确认原生行为**：函数体、实际 pop/write、派发表或 xref 支持。
2. **已实现并测试**：有当前 Rust 实现和列出的回归用例，但不自动代表原生所有边界完全一致。
3. **移植近似/未完成**：明确记录在各篇边界和下方缺口中。

`extsig.rs` 的 `Blocked` 指签名/语义尚未充分验证，不能由此断言 VM 没有 handler。
旧 IDA 人工名称存在错误，尤其 Select 表；地址 + 函数体优先于名称。

## 阅读顺序

| 主题 | 文档 | 内容 |
|---|---|---|
| 总架构/数据库 | [原始索引](RE_INDEX.md) | Game/PAL 分层、主循环、关键表地址 |
| VM/指令 | [GAME_VM_RE](GAME_VM_RE.md) | 主指令、分类扩展调用、dispatch 地址；本轮更正 Select 表 |
| 参数协议 | [PARAMETER_PASSING_RE](PARAMETER_PASSING_RE.md) | LIFO、返回目标、task_data 与 ctx、ABI |
| 内存布局 | [ENGINE_CONTEXT_RE](ENGINE_CONTEXT_RE.md) | 已知字段偏移；旧标题 Complete 表示汇总已知项，不保证无未知字段 |
| 结构体 | [STRUCTS_RE](STRUCTS_RE.md) | sprite/task/action/声音等字段与 xref |
| PAL 接口 | [PAL_EXPORTS_RE](PAL_EXPORTS_RE.md) | debug/release 导出与引擎边界 |
| Select | [softpal-select](softpal-select.md) | 新恢复的渲染、阻塞、分支、回调、参数和剩余近似 |
| runtime 补充 | [softpal-runtime-audit](softpal-runtime-audit.md) | voice autopan/MIX、Tips、messagebox、save 审查 |
| 存档槽 | [save-slot-semantics](save-slot-semantics.md) | -1 wildcard、continue、新存档标记、title 查询 |
| 多声道语音 | [voice-mix-native](voice-mix-native.md) | MIX 单独 lane，音量、mute、pan 和 flag |
| UI/提示审查 | [native-messagebox-tips-audit](native-messagebox-tips-audit.md) | FFI 弹窗生命周期、Tips 可达性 |
| WMV/ASF | [softpal-wmv-decoder](softpal-wmv-decoder.md) | siglus_rs 复用、payload/codec 状态和支持边界 |
| MPEG | [softpal-mpeg-decoder](softpal-mpeg-decoder.md) | siglus_rs 复用、PS/PTS/seek/loop 和测试 |
| 签名目录 | [EXTCALL_CATALOG](EXTCALL_CATALOG.md) | 当前源码中的显式签名、pop、参数、可信状态；不是完整 native extcall 表 |

## 引擎运行路径

```text
资源启动: loose → data.pac → ARCHIVE.DAT / 语言 archive list
    ↓
CoreAssets: Script.src + Point.dat + Mem.dat + File.dat + Text.dat
    ↓
Game VM: fetch opcode → resolve operands → extcall(category,index,destination)
    ├─ text/font/history/tips → text state → cached sprite surfaces
    ├─ select/button/menu → input → wait/reaction/callback/point branch
    ├─ sprite/effect/movie → scene tree → CPU / wgpu renderer
    ├─ BGM/SE/voice → decode/mix/channel gain/pan → audio backend
    └─ save/load → native prefix + portable checkpoint → scene reconstruction
```

Sv20 指令为 little-endian DWORD；高 16 位为 1，低 16 位为 opcode。
23 为 extcall，随后是 `category << 16 | index` 和结果目标 DWORD。
不能将所有目录名称 35..262 当成目标脚本实际使用的 primary opcode。
此目标实际遍历得到 67,196 次 extcall；本轮 Tips 命令调用数为 0。
入口 0x1F8，Point.dat 为反向 point 索引，4096 项。操作数寻址和 point 协议见源码与参数文档。

## 实现位置

| 系统 | 当前源码 |
|---|---|
| 指令/操作数/签名 | `crates/pal-script/src/{opcodes,operand,extsig,extsig_auto}.rs` |
| 资源/归档启动 | `crates/pal-asset/src/resource.rs` |
| VM/文字/声音/保存 | `crates/pal-vm/src/runtime.rs` |
| 选择/Tips | `crates/pal-vm/src/runtime/{select,tips}.rs` |
| 字体/输入/声音 | `crates/pal-vm/src/{font,input,audio}.rs` |
| sprite/movie/渲染 | `crates/pal-vm/src/{sprite,msprite}.rs`、`renderer/` |
| 原生存档前缀 | `crates/pal-vm/src/save_format.rs` |
| 宿主桥接 | `crates/pal-vm/src/ffi.rs` |

## 本轮前后变化

- Select 从“存储选项后直接继续”改为实际绘制并等待输入，原生分支和反应回调有回归。
- voice autopan 从只存配置改为参与播放及每帧位置更新；pan 对应 DirectSound 衰减。
- voice MIX 保留每个角色的独立音量/mute/pan，组等待/释放不误包含独立 slot 8。
- Tips 补齐 36..46 的命令语义和正确 pop 数；同步反编译签名，避免吞掉后续调用参数；producer/callback and save integration now follow [port-corrections.md](port-corrections.md).
- messagebox 宿主回调接到 fatal frame 路径，异步回执/注销/销毁有 mailbox 保护。
- is_save(-1) 原本已有 wildcard；本轮修正其他负数、最近槽初值及删除失败后的标记。
- 电影推进按 elapsed time/PTS，保留未来帧和循环余量；修复 ASF 同包后续 payload 丢失。
- 按用户指定复用 siglus_rs，来源提交和 codec 边界详见解码文档。

## 明确未完成的部分

| 缺口 | 当前事实 / 下一步证据 |
|---|---|
| Tips remaining boundaries | Producer, reveal colors, script callbacks and v10 save restoration are implemented; native typography and final-frame callback ordering remain unverified. |
| 原生存档完整互通 | sprite/history/sound wrapper 等镜像未全映射；原引擎接受移植写入文件未端到端证明 |
| MP4 | Rejected; verified release support is not established. The separate debug H.264 graph is incomplete; see [audit](mp4-native-audit.md). |
| Select 全边界 | 系统 cursor warp、中间态保存恢复等见专文；push/pull 输入模式已接通 |
| 文字排版 | 禁则、溢出省略、backlog voice icon 等既有缺口仍存在 |
| Audio/host boundaries | See the current BGV and voice registry notes; not all native error-dialog sites are mapped. |
| codec 完备性 | 有受支持子集和显式 Unsupported；不能把通过少量样本当全格式认证 |

## 验证记录

最终源码合并后验证：

| 测试 | 结果 |
|---|---|
| `cargo test -p pal-vm --lib` | 83 passed / 4 ignored |
| `cargo test -p pal-vm --test movie_timing --test mpeg2_movie --test save_semantics_fixture` | 13 passed |
| `cargo test -p pal-script` | 11 passed |
| `cargo test -p pal-decompiler` | 29 passed |
| `cargo test -p wmv-decoder --lib` | 15 passed |
| `cargo check -p wmv-decoder --no-default-features --features video-only` | passed |
| `git diff --check` | passed |

上述互不重复的测试共 **151 passed**。Select 本地只读游戏 fixture 在本轮另行显式运行通过。
没有声称在 Windows/Android/iOS 实机或 Rust 1.87 工具链完成验证；当前测试工具链为 Rust 1.95。

仓库已有已知限制：`cargo test --workspace --all-targets` 会因缺失
`testcase/SYSTEM.INI` 的 nls_config_fixture 失败并中断后续集成二进制。应使用 focused crate/test 命令。
真实 WMV fixture 默认 ignore，需要本机资源；原创微型媒体 fixtures 可独立执行，不依赖游戏数据。

## 复现 IDA 证据

脚本：[ida_dump_functions.py](evidence/ida_dump_functions.py)。在 IDB 副本上执行，设
`IDA_EVIDENCE_OUT`、`IDA_EVIDENCE_EAS`（逗号地址）、`IDA_EVIDENCE_TABLES`（地址:项数）。
脚本等待 auto-analysis 后导出伪代码/反汇编/xref，不改名、不加注释、不写类型。

本机 IDA 初始 Python 路径指向旧 Homebrew 版本导致崩溃；本轮使用隔离的 IDAUSR +
idapyswitch 指向当前 Python 3.11，避免改动用户全局 IDA 配置。临时数据库及运行日志在
`target/reverse-audit/`；长期必要证据已复制到本目录。二进制和 IDB 保持 Git 忽略，Markdown 和证据 JSON/Python 可入库。
