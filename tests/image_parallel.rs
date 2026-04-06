use jail_ai::image_parallel::{build_language_layers_parallel, ensure_layer_exists};
use std::collections::HashMap;

#[test]
fn test_parallel_build_empty() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let result: Result<HashMap<String, String>, _> =
            build_language_layers_parallel("base", &[], &[], false, false).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    });
}

#[tokio::test]
async fn test_ensure_layer_exists_unknown() {
    let result: Result<(), _> = ensure_layer_exists("unknown-layer", None).await;
    assert!(result.is_ok());
}
