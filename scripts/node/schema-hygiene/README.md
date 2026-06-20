# Schema Hygiene Rules

## Profiles

- `managed_table` is the default for every unmarked physical table.
- `dynamic_model_table` is for generated workspace-scoped user data tables.
- `registered_system_table` is a fixed physical table registered into metadata.

## registered_system_table

`registered_system_table` means fixed physical table, metadata registration, and read-only field template.

It is not a schema hygiene exemption. The scanner still checks required physical columns, primary key shape, scope, indexes, constraints, and parse failures. If an existing registered system table misses a fixed-template requirement, the gate must report the difference first; schema repair belongs in a separate migration issue.

Metadata systems may manage display configuration, actions, views, and relation metadata for these tables, but must not add, delete, rename, or physically change registered system table columns.
