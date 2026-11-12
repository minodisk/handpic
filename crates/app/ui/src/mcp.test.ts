import { describe, expect, test } from "vitest";
import { mcpEndpoint, mcpExamples, mcpStatus } from "./mcp.js";

describe("mcpEndpoint", () => {
  test("is the loopback /mcp URL on the port", () => {
    expect(mcpEndpoint(41917)).toBe("http://127.0.0.1:41917/mcp");
  });
});

describe("mcpStatus", () => {
  test("shows the endpoint while listening", () => {
    expect(mcpStatus({ enabled: true, port: 41917, error: null })).toBe(
      "Listening on http://127.0.0.1:41917/mcp",
    );
  });

  test("shows Off when disabled", () => {
    expect(mcpStatus({ enabled: false, port: 41917, error: null })).toBe("Off");
  });

  test("shows the bind error", () => {
    const error = "could not listen on 127.0.0.1:41917: Address in use";
    expect(mcpStatus({ enabled: true, port: 41917, error })).toBe(error);
  });
});

describe("mcpExamples", () => {
  test("adds the endpoint to Claude Code over HTTP", () => {
    expect(mcpExamples(41917)[0]).toEqual({
      client: "Claude Code",
      text: "claude mcp add --transport http handpic http://127.0.0.1:41917/mcp",
    });
  });

  test("relays Claude Desktop through mcp-remote", () => {
    const desktop = mcpExamples(41917)[1];
    expect(desktop?.client).toBe("Claude Desktop (claude_desktop_config.json)");
    expect(JSON.parse(desktop?.text ?? "")).toEqual({
      mcpServers: {
        handpic: {
          command: "npx",
          args: ["-y", "mcp-remote", "http://127.0.0.1:41917/mcp"],
        },
      },
    });
  });
});
