use app_tauri_lib::demo_render_scene;

#[test]
fn builds_phase_7_demo_render_scene_for_desktop_viewer() {
    let scene = demo_render_scene(42);

    assert_eq!(scene.scheme_id, 42);
    assert_eq!(scene.dimensions, [8, 5, 8]);
    assert_eq!(scene.stages.len(), 2);
    assert_eq!(scene.blocks.len(), 9);
    assert_eq!(scene.chunks.len(), 2);
    assert!(scene.chunks.iter().all(|chunk| chunk.face_count > 0));
    assert!(scene.blocks.iter().any(|block| block.stage_id.is_none()));
}
