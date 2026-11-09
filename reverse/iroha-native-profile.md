# Iroha (trenka.exe) native interface profile

Scope: the `trenka.exe` / `dll/Pal.dll` pair whose hashes are recorded in
`reverse/evidence/iroha-trenka-interface-inventory.json`. Addresses below are
only valid for that build. The inventory records interface layout only; a
present slot does not mean Sena implements the handler's semantics.

## Native evidence

- `trenka.exe` binds `PAL.dll` through the delay-load directory (213 names);
  it has no ordinary PAL import entries.
- `dll/Pal.dll` exports 285 names. Against the audited Reference DLL it adds
  only `?PalGetlpPool@@YAPAEXZ` and lacks the six language/position exports
  (`PalGetLanguage`, `PalSetLanguage`, `PalSystemCNPathSet`,
  `PalSystemTCPathSet`, `PalSpriteGetPosX`, `PalSpriteGetPosY`). It lacks the
  registered-dispatch markers `PalSystemPathInit` and
  `PalGetChangeWindowModeTime`, so the registered profile stays disabled.
- Primary opcode table: `0x64C890`, 265 slots, no bounds check. Extcall
  category table: `0x626098` (24 entries), dispatcher `0x42D920`; a null
  category returns 0 without writing the destination.
- Primary alias regions differ from Reference: starts are 23:35, 10:40, 5:82,
  6:98, 9:110, 12:171, 2:176, 13:226, 7:254 (Reference: 5:80, 6:96, 9:108,
  13:224, 7:252, length 263). `primary_aliases.rs` encodes the Reference
  layout, so high primary opcodes would be mis-routed for this build. The
  shipped script only uses primary opcodes 1..33, so this is latent.
- Category 18 slots 88/89/91/96 have the registered-build meanings
  (network status, touch availability, touch enable, `Pal3DIs`) even though
  the registered DLL markers are absent. Sena gates them on
  `registered_softpal`; a capability that is not tied to those exports is
  still needed.
- `gosub_point` (`0x42D7B0`) resolves its target through the shared operand
  resolver `0x42D2E0`, as do primary opcodes 1..19, 26..28 and 30..33. The
  resolver returns the raw word for top nibble 0, so plain point ids are
  unchanged; the script also emits `gosub var[1]` (`0x40000001`).

## Implementation state

- The VM always decodes the gosub target as an operand; previously only the
  registered profile did. Plain ids keep the old behaviour. Test:
  `gosub_resolves_variable_point_operand_without_registered_profile`.
- The disassembler emits `PointOperand` for non-immediate gosub targets
  instead of failing the `Point.dat` range check.

## Runtime validation

- Headless boot reaches the title screen through the logo movie.
- The dynamic gosub site was not executed within the first 2100 frames, so
  the fix is validated by native evidence and the unit test only.
