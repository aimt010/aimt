pub const AIMT_MARKER: &str = "## aimt";

pub const AIMT_HOST_GUIDE_MD: &str = concat!(
    "## aimt\n\n",
    "This project uses AIMT (AI Mapping Taxonomy) with `.aimt` files. See `.agents/aimt/index.md` for full guidance.\n\n",
    "AIMT is the project's primary knowledge map when available; the source project is evidence. When AIMT knowledge is missing, stale, or insufficient, inspect source evidence and update AIMT when appropriate.\n\n",
    "Do not create a separate AIMT project directory — `.aimt` is the canonical package; `.agents/aimt/*` are only instructions; source files are evidence.\n\n",
    "@./.agents/aimt/index.md\n"
);

// Legacy full guide kept for reference but no longer canonical source.
// New canonical source is `src/plugins/installer/aimt/*`.
pub const AIMT_GUIDE_MD: &str = concat!("## aimt\n\n", include_str!("guide_body.md"));
