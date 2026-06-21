alter table application_run_trace_projection_statuses
    add column if not exists id uuid;

alter table application_run_trace_projection_statuses
    add column if not exists scope_id uuid;

alter table application_run_trace_projection_statuses
    add column if not exists created_by uuid;

alter table application_run_trace_projection_statuses
    add column if not exists updated_by uuid;

update application_run_trace_projection_statuses statuses
   set scope_id = flow_runs.scope_id
  from flow_runs
 where statuses.flow_run_id = flow_runs.id
   and statuses.scope_id is null;

with generated as (
    select
        flow_run_id,
        projection_version,
        md5(
            '1flowbase.application_run_trace_projection_status:'
            || flow_run_id::text
            || ':'
            || projection_version::text
        ) as generated_id
    from application_run_trace_projection_statuses
    where id is null
)
update application_run_trace_projection_statuses statuses
   set id = (
       substr(generated.generated_id, 1, 8)
       || '-'
       || substr(generated.generated_id, 9, 4)
       || '-'
       || substr(generated.generated_id, 13, 4)
       || '-'
       || substr(generated.generated_id, 17, 4)
       || '-'
       || substr(generated.generated_id, 21, 12)
   )::uuid
  from generated
 where statuses.flow_run_id = generated.flow_run_id
   and statuses.projection_version = generated.projection_version
   and statuses.id is null;

alter table application_run_trace_projection_statuses
    alter column id set not null;

alter table application_run_trace_projection_statuses
    alter column scope_id set not null;

create index if not exists application_run_trace_projection_statuses_scope_created_id_idx
    on application_run_trace_projection_statuses (scope_id, created_at, id);

create index if not exists application_run_trace_projection_statuses_scope_status_updated_flow_run_idx
    on application_run_trace_projection_statuses (scope_id, status, updated_at, flow_run_id);

alter table application_run_trace_nodes
    add column if not exists id uuid;

alter table application_run_trace_nodes
    add column if not exists scope_id uuid;

alter table application_run_trace_nodes
    add column if not exists created_by uuid;

alter table application_run_trace_nodes
    add column if not exists updated_by uuid;

update application_run_trace_nodes nodes
   set scope_id = flow_runs.scope_id
  from flow_runs
 where nodes.flow_run_id = flow_runs.id
   and nodes.scope_id is null;

with generated as (
    select
        trace_node_id,
        md5('1flowbase.application_run_trace_node:' || trace_node_id::text) as generated_id
    from application_run_trace_nodes
    where id is null
)
update application_run_trace_nodes nodes
   set id = (
       substr(generated.generated_id, 1, 8)
       || '-'
       || substr(generated.generated_id, 9, 4)
       || '-'
       || substr(generated.generated_id, 13, 4)
       || '-'
       || substr(generated.generated_id, 17, 4)
       || '-'
       || substr(generated.generated_id, 21, 12)
   )::uuid
  from generated
 where nodes.trace_node_id = generated.trace_node_id
   and nodes.id is null;

alter table application_run_trace_nodes
    alter column id set not null;

alter table application_run_trace_nodes
    alter column scope_id set not null;

create index if not exists application_run_trace_nodes_scope_created_id_idx
    on application_run_trace_nodes (scope_id, created_at, id);

create index if not exists application_run_trace_nodes_scope_flow_parent_order_idx
    on application_run_trace_nodes (scope_id, flow_run_id, parent_trace_node_id, order_key, trace_node_id);

create index if not exists application_run_trace_nodes_scope_flow_owner_idx
    on application_run_trace_nodes (scope_id, flow_run_id, owner_kind, owner_id);

alter table application_run_trace_node_contents
    add column if not exists id uuid;

alter table application_run_trace_node_contents
    add column if not exists flow_run_id uuid;

alter table application_run_trace_node_contents
    add column if not exists scope_id uuid;

alter table application_run_trace_node_contents
    add column if not exists created_by uuid;

alter table application_run_trace_node_contents
    add column if not exists updated_by uuid;

update application_run_trace_node_contents contents
   set flow_run_id = nodes.flow_run_id,
       scope_id = nodes.scope_id
  from application_run_trace_nodes nodes
 where contents.trace_node_id = nodes.trace_node_id
   and (contents.flow_run_id is null or contents.scope_id is null);

with generated as (
    select
        trace_node_id,
        md5('1flowbase.application_run_trace_node_content:' || trace_node_id::text) as generated_id
    from application_run_trace_node_contents
    where id is null
)
update application_run_trace_node_contents contents
   set id = (
       substr(generated.generated_id, 1, 8)
       || '-'
       || substr(generated.generated_id, 9, 4)
       || '-'
       || substr(generated.generated_id, 13, 4)
       || '-'
       || substr(generated.generated_id, 17, 4)
       || '-'
       || substr(generated.generated_id, 21, 12)
   )::uuid
  from generated
 where contents.trace_node_id = generated.trace_node_id
   and contents.id is null;

alter table application_run_trace_node_contents
    alter column id set not null;

alter table application_run_trace_node_contents
    alter column flow_run_id set not null;

alter table application_run_trace_node_contents
    alter column scope_id set not null;

create index if not exists application_run_trace_node_contents_scope_created_id_idx
    on application_run_trace_node_contents (scope_id, created_at, id);

create index if not exists application_run_trace_node_contents_scope_flow_created_trace_idx
    on application_run_trace_node_contents (scope_id, flow_run_id, created_at, trace_node_id);
