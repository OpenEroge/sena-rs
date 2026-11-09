# SoftPAL port acceptance status

Updated 2026-10-24. **Complete behavioral compatibility is not established.**
This document separates implementation, local validation, and remaining work.
Export counts, handler counts, and successful cross-builds are not compatibility
percentages. No new regression tests were added in the console CI follow-up.
Its existing host-audio library checks passed 252 tests (5 ignored) after correcting
four stale fixture/clock/state assumptions; this is packaging-gate validation,
not new native-behavior or full-route evidence. See [CI scope](../platform/README.md#continuous-integration).

## Current implementation

- [Console emulator execution](console-emulator-validation.md) now covers actual
  Ryujinx/Vita3K packages. Switch's unused libc patch and a Vita BGM allocation
  failure were reproduced and corrected. The newer main-story replay also
  exposed a missing four-argument substring wrapper that corrupted the VM stack
  and caused a 2 GiB allocation, plus a literal-only gosub target error;
  [ADR-0008](adr/0008-preserve-newer-substring-stack-and-gosub-operands.md)
  records the native contracts and corrections. Independent Vita GPU budgets and
  separate Vita large-block allocation ([ADR-0011](adr/0011-separate-vita-large-allocations.md)),
  two-pass static audio decoding, shared pixel allocations and streaming GE/PGD3
  prediction reduce transient memory. Vita group-3 Ogg BGM uses bounded
  streaming on both resource and VM loader paths ([ADR-0010](adr/0010-stream-vita-ogg-bgm.md)); emulator results do
  not certify hardware or full-route behavior. See [ADR-0007](adr/0007-verify-console-runtime-and-memory.md).
- [Diagnostic checkpoints](adr/0012-host-diagnostic-checkpoints.md) now restore
  full shared sprite pixels and live VM contexts through host APIs. The final
  Vita package passed 400-frame and 240-frame restored segments, including six
  4096-square image allocations, within tracked CPU/GPU budgets and with tracked
  large-block/texture reservations released afterward. These finite segments
  replace repeated title replays for targeted diagnosis; they do not establish
  complete route or hardware acceptance.
- [Accepted decisions](adr/README.md) retain confirmed contracts and explicit
  reopening rules. [Standing-art sizing](adr/0002-preserve-native-sprite-size.md)
  now preserves unscaled source dimensions, script scale and centered drawing;
  the previous automatic height fit and whole-rectangle projection were incorrect.
  See the [targeted validation](evidence/standing-size-validation.json) for both
  games, including the pre-existing newer-script failure beyond the capture scene.
- [Native ABI declarations](abi/README.md): 148 declaration records generated
  into C and Rust, with checked addresses and binary hashes. Per-build matching
  declarations: Koikake 108, reference 111, totsulover 142.
- [Export audit](contracts/README.md): all 921 exported-name/build rows have
  boundary evidence (262 Koikake, 290 reference, 369 totsulover). Private
  callees, source-level types, and runtime side effects are not all resolved.
- The newer registration catalog has 95 individually reviewed VM wrappers out
  of 787 distinct registered coordinates. The remaining 692 are unreviewed for
  that build; this is not a count of missing Rust handlers. Legacy `Blocked`
  signature labels also mean unverified signatures, not absent handlers.
- [Buttons](input-buttons-contract.md): raw flags, native hit selection,
  independent group hover, delayed click/sound completion, hover callbacks,
  timed locks, source rectangles, animation clearing, and stored query latches.
  Newer reaction/toggle/info queries are connected. The seven extensions `8:42..48`
  now implement input gates, attachment transforms and status/hit queries. Verified slider paths use
  separate wrapper base/offset words and preserve native arithmetic instead
  of neighbor-position inference. Native control returns can end a VM slice
  without marking the call unsupported or overwriting its destination.
- [Tasks](core-contract.md): registered portable callback adapters, work/current
  context, relative sibling insertion, destructor access, native wait initialization
  and wrapping time. Foreign x86 callbacks are not executed as host addresses.
- [Graphics](render-mode-implementation.md): native blend modes and sticky draw
  state across CPU/wgpu/deko3d/GXM paths, reviewed channel-copy behavior,
  sprite-backed target lists and newer child-target commands. Child propagation
  uses ordered traversal and additive transform/color deltas rather than repeated
  alpha multiplication and accumulating Z. Packed button existence and relative
  position queries now share the recovered tag/group/index interpretation.
  Full wrapper-bank parity remains open. Follow-up review also found a confirmed
  newer `3:4` arity/placement mismatch still requiring correction.
- [HiDPI presentation](hidpi-presentation.md): letterboxed PAL viewport shared by
  both renderers and the cursor mapping; ADV, name and backlog text rasterized at
  the window's text density as generation-bound dense surface copies.
- [Audio](bgm-byte-range-contract.md): BGM raw byte ranges, per-build flags and
  fades, reserved streaming records, native command queries, engine-clock fade
  envelopes, separate ratio storage, and known source buffer-format metadata.
  [Movie audio](movie-audio-contract.md) is connected to VM/scene playback.
  MPEG/WMV decoders are integrated; MP4 work remains excluded by user instruction.
- Select polling now yields the VM slice without allocating an extra PAL wait
  task; one-frame clicks reach the choice handler. ADV wrapping/reveal measures
  complete native markup units instead of truncating tags or escapes.
- Portable save version 15 retains earlier readers. Versions 12/13 add native
  button state, independent scale, child attachments and target ordering; version
  14 adds button group toggles and wrapper base/slider offsets. Version 15 adds
  button attachments/input gates, history icon configuration/record metadata, and
  the dynamic-string bank/cursor. History checkpoint navigation uses portable
  checkpoint files and a native-style replay-until-PC gate.
- [Native Koikake save import](native-save-import.md) now traverses the recovered
  conditional record stream, restoring user memory, operand/argument/return
  stacks, dynamic strings, static scenes, button callbacks, backlog, ADV state,
  and BGM/BGV names. The old fixed-offset text heuristic was wrong. Native
  import is selected only by the exact audited PAL/Game hash pair.
- Exact build identity uses both SHA-256 and file size for PAL and the executable.
  Missing, modified, mixed, or ambiguous pairs remain `Unknown`; capability
  detection remains separately scoped and does not prove binary identity.

Earlier work is documented in [action scheduling](action-scheduler.md),
[scene transitions](scene-transitions.md), [MPEG](softpal-mpeg-decoder.md),
[WMV](softpal-wmv-decoder.md), and [platform builds](../platform/README.md).

## Remaining acceptance boundaries

| Area | Known remaining implementation or verification |
| --- | --- |
| Native ABI and VM | Uncertain source types/returns and unreviewed newer wrappers remain. Export-boundary review is not a full private-call-graph reconstruction. Native callback addresses are represented by explicit portable adapters. |
| Graphics | Arbitrary foreign render callbacks, full wrapper animation banks, every integer/float rotation quantization stage, radial/bicubic dispatch, and Vita GPU raster integration remain open. Vita uses a CPU raster fallback. |
| Native saves | No original-engine save fixture is available for exercising the new importer. Unsupported active effects/processes/SE and other auxiliary state are diagnosed. Portable snapshots omit native rotation/center/render-mode state across subsequent restart. Writing complete original-engine mirror blocks is unverified. |
| Audio | DirectSound ring cursors and resource addresses, EPEG cursor/silence-tail behavior, nonstandard streaming formats, exact restored positions/ranges/gain, and device timing remain incomplete. |
| Movies | Device recreation/error recovery, decoder coverage, hardware scheduling, and bounded decoded-audio memory require further work or comparison. |
| Text | Native GDI glyph raster parity, kinsoku and overflow ellipsis remain open. History icon rendering/callbacks and checkpoint replay are connected but need full native comparison. The former startup gaps `14:13` and `14:24` now execute. |
| Transitions | Capture timing, interruption, callback edge cases, and all numerical kernels need native frame comparison. |
| Platforms | Cross-built packages do not prove console hardware or mobile lifecycle compatibility. No complete game route has been compared against native traces/frames. |

The table lists known boundaries, not proof that everything else is identical.
`app_exec` remains the repository-authorized no-op.

## Validation

This UI batch is recorded in [ui-extensions-validation.json](evidence/ui-extensions-validation.json). Earlier platform/native checks are in [native-followup-validation.json](evidence/native-followup-validation.json).
This batch passes host release and Win32-x86 compilation and declaration freshness
checks. Koikake reaches ADV in an isolated 4,200-frame run. The host FFI run also
produced PCM and remained silent after teardown; exact counts and artifact hashes
are in the batch record. The newer trial completes the title/voice/chapter click
flow and reaches ADV in 6,200 fixed 16 ms frames. Its previously invisible title
buttons and dropped selection clicks are corrected from native evidence.
These short runs do not establish whole-game parity. C/C++ ABI and console package results in that earlier record predate the UI
batch. Subsequent actual native-package execution is documented separately in
the [dated console report](console-emulator-validation.md), with its exact
artifacts, replay limits and remaining hardware boundaries. Reproduction commands (platform rebuilds listed for
reference; consult the dated records for what actually ran):

```sh
python3 reverse/abi/generate.py --check
python3 reverse/evidence/generate_registered_dispatch.py --check
python3 reverse/contracts/generate_inventory.py --check --require-all-reviewed
cargo check -p pal-vm --lib --features host-audio
cargo check -p pal-vm --lib --no-default-features --target i686-pc-windows-gnu
i686-w64-mingw32-gcc -std=c11 -Wall -Wextra -Werror -fsyntax-only -x c reverse/abi/pal32.h
i686-w64-mingw32-g++ -std=c++11 -Wall -Wextra -Werror -fsyntax-only -x c++ reverse/abi/pal32.h
platform/switch/build_switch.sh
platform/vita/build_vita.sh
```

Original game files remain read-only. Writable saves, resource symlinks,
copied IDA databases, logs, and diagnostic captures stay under
`target/reverse-audit/`. Earlier test totals in older validation records are
historical and do not certify this newer source revision.
