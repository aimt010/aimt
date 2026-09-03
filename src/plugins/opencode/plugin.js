import { tool } from "@opencode-ai/plugin";
import { spawnSync } from "child_process";
import { join, isAbsolute } from "path";

export const AimPlugin = async ({ directory, worktree }) => {
  return {
    tool: {
      aimt: tool({
        description: "AIMT workflow — discover, open, search, read, follow hierarchy/files/relations, validate. Uses AIMT Core via Store and WorkflowEngine.",
        args: {
          action: tool.schema.enum(["search", "read", "follow_parent", "follow_file", "follow_relation", "validate"]).describe("AIMT capability to execute"),
          aimt: tool.schema.string().optional().describe("Path to .aimt file or directory containing .aimt, defaults to worktree"),
          query: tool.schema.string().optional().describe("Search query for search action"),
          id: tool.schema.string().optional().describe("Entity id for read/follow actions"),
        },
        async execute(args, context) {
          const aimtPath = args.aimt
            ? (isAbsolute(args.aimt) ? args.aimt : join(context.directory, args.aimt))
            : null;
          // Map underscore actions to hyphen for new CLI (follow_parent -> follow-parent)
          const cliAction = args.action.replace(/_/g, "-");
          const binArgs = ["run", "--quiet", "--bin", "aimt", "--", cliAction];
          if (aimtPath) binArgs.push(aimtPath);
          if (args.query) binArgs.push("--query", args.query);
          if (args.id) binArgs.push(args.id);
          const result = spawnSync("cargo", binArgs, {
            cwd: context.worktree || directory,
            encoding: "utf-8",
            timeout: 10000,
          });
          if (result.error) return `Error invoking AIMT: ${result.error.message}`;
          const output = (result.stdout || "").trim();
          const stderr = (result.stderr || "").trim();
          if (output) {
            try { return JSON.stringify(JSON.parse(output), null, 2); } catch { return output; }
          }
          if (stderr) return `Error: ${stderr}`;
          return "No output from AIMT";
        },
      }),
    },
  };
};

export default AimPlugin;
export const AimtoolPlugin = AimPlugin;
