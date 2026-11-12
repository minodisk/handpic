// The payload of the `mcp-state` event and the `mcp_enabled` command.
export interface McpState {
  enabled: boolean;
  port: number;
  error: string | null;
}

export interface McpExample {
  client: string;
  text: string;
}

export function mcpEndpoint(port: number): string {
  return `http://127.0.0.1:${port}/mcp`;
}

export function mcpStatus({ enabled, port, error }: McpState): string {
  if (error !== null) return error;
  return enabled ? `Listening on ${mcpEndpoint(port)}` : "Off";
}

// Examples for two MCP clients; any client that speaks Streamable HTTP only
// needs the endpoint. Claude Desktop's config takes stdio servers, so it goes
// through the `mcp-remote` relay.
export function mcpExamples(port: number): McpExample[] {
  const endpoint = mcpEndpoint(port);
  const desktop = {
    mcpServers: {
      handpic: { command: "npx", args: ["-y", "mcp-remote", endpoint] },
    },
  };
  return [
    { client: "Claude Code", text: `claude mcp add --transport http handpic ${endpoint}` },
    {
      client: "Claude Desktop (claude_desktop_config.json)",
      text: JSON.stringify(desktop, null, 2),
    },
  ];
}
