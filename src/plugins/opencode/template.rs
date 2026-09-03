pub const OPENCODE_PLUGIN_JS: &str = include_str!("plugin.js");

pub const OPENCODE_AIMT_COMMAND_MD: &str = r#"---
description: AIMT mapping — build or update the .aimt knowledge package
---

Read `.agents/aimt/index.md`.

If `$ARGUMENTS` is ".":
  read `.agents/aimt/mapping.md`
  follow its instructions.

If `$ARGUMENTS` is "update":
  read `.agents/aimt/update.md`
  follow its instructions.

Otherwise:
  report that the AIMT command only supports `/aimt .` and `/aimt update`.
"#;
