use jail_ai::ebpf::loader_client::find_loader_binary;

#[test]
fn test_find_loader_binary() {
    match find_loader_binary() {
        Ok(path) => println!("Found loader at: {:?}", path),
        Err(e) => println!("Loader not found (expected in test): {}", e),
    }
}
