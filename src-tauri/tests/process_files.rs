use tether_lib::core;

#[test]
fn process_files_json_shape() {
    let resp = core::process_paths(vec!["/definitely/not/here".into()]);
    // Check minimal shape without relying on private fields elsewhere
    assert_eq!(resp.total, 1);
    assert_eq!(resp.processed, 0);
    assert_eq!(resp.files.len(), 1);
}
