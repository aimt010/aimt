pub const CURSOR_RULE_MDC: &str = concat!(
    include_str!("header.mdc"),
    "\n",
    "This project uses AIMT with `.aimt` files. See `.agents/aimt/README.md` for full guidance.\n\n",
    "When to use AIMT: before reading source. Operations: discover, search, read, follow_parent, follow_file, relation, validate, update, hosted vs install, permission, project-knowledge, workflow.\n\n",
    "Hosted: `.aimt` package is portable read-only; install is development. Do not modify hosted.\n\n",
    "@./.agents/aimt/README.md\n",
    "@./.agents/aimt/knowledge.md\n",
    "@./.agents/aimt/operations.md\n",
    "@./.agents/aimt/schema.md\n",
    "@./.agents/aimt/security.md\n"
);
