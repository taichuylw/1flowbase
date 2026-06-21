use access_control::permission_catalog;

#[test]
fn permission_catalog_includes_mcp_management_resource() {
    let codes = permission_catalog()
        .into_iter()
        .map(|permission| permission.code)
        .collect::<Vec<_>>();

    assert!(codes.contains(&"mcp_management.view.all".to_string()));
    assert!(codes.contains(&"mcp_management.manage.all".to_string()));
}
