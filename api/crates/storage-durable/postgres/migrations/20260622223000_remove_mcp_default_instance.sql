drop index if exists mcp_instances_workspace_default_idx;

alter table mcp_instances
    drop column if exists is_default;
