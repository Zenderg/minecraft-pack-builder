# Ghost Rendering And Registry Safety

This document is the source of truth for in-world guide rendering invariants, modded-block fallback
behavior, and registry lookup safety in the MPB Minecraft runtime. User-visible guide behavior
belongs in the [product contract](../../../docs/product/patcher-and-minecraft-mod.md), while loader
build and runtime lifecycle details belong in the [module README](../README.md).

## Rendering Invariants

- Ordinary block ghosts use Minecraft client rendering rather than an approximate external renderer.
- Unsupported orientation rotation is rejected before saving, so a mutation cannot leave a partially rotated scheme.
- The guide preserves the `RenderType` selected by Minecraft's renderer and only wraps the returned `VertexConsumer` to apply guide alpha.
- Forcing every block through a generic translucent render type is invalid because it breaks texture and material lookup for block-entity-style vanilla blocks and modded special renderers.

## Modded-Block Fallback

A valid registry id, block state, and `renderSingleBlock` call do not guarantee visible pixels.
Create cogwheels reproduced this in an All of Create - Aeronautics NeoForge client: guide outlines
were positioned correctly, while the kinetic block-entity rendering path produced no visible
block-model ghost.

For a non-vanilla block whose render shape is not `MODEL`, whose state has a block entity, or whose
baked model is custom, missing, or has no quads, the guide keeps the normal block render attempt and
also renders an alpha-wrapped item-model fallback. This is a generic modded-block fallback, not a
Create-specific special case.

## Registry Lookups

Runtime block lookups check explicit registry membership before calling
`BuiltInRegistries.BLOCK.get(...)`. Minecraft may otherwise return a fallback block for an unknown
modded id, causing MCP clients to receive a false "known block with no properties" response.

Real-client rendering for ordinary blocks, block entities, custom baked models, and fallback item
models is required by the [patcher and runtime release checklist](../../../docs/validation/patcher-runtime-release-checklist.md).
