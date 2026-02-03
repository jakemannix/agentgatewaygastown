"use client";

import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SourceTool, Server } from "@/lib/registry-types";

interface SourceToolEditorProps {
  source: SourceTool;
  servers: Server[];
  onChange: (updates: Partial<SourceTool>) => void;
}

export function SourceToolEditor({ source, servers, onChange }: SourceToolEditorProps) {
  const addDefault = () => {
    const newDefaults = { ...(source.defaults || {}), "": "" };
    onChange({ defaults: newDefaults });
  };

  const updateDefault = (oldKey: string, newKey: string, value: unknown) => {
    const newDefaults = { ...(source.defaults || {}) };
    if (oldKey !== newKey) {
      delete newDefaults[oldKey];
    }
    newDefaults[newKey] = value;
    onChange({ defaults: newDefaults });
  };

  const removeDefault = (key: string) => {
    const newDefaults = { ...(source.defaults || {}) };
    delete newDefaults[key];
    onChange({ defaults: newDefaults });
  };

  const addHideField = (field: string) => {
    if (field && !source.hideFields?.includes(field)) {
      onChange({ hideFields: [...(source.hideFields || []), field] });
    }
  };

  const removeHideField = (field: string) => {
    onChange({ hideFields: source.hideFields?.filter((f) => f !== field) });
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>Backend Tool Reference</CardTitle>
          <CardDescription>Configure which backend tool this virtual tool maps to</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="server">Server / Target *</Label>
              <Select
                value={source.server || source.target || ""}
                onValueChange={(v) => onChange({ server: v, target: undefined })}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select server" />
                </SelectTrigger>
                <SelectContent>
                  {servers.map((server) => (
                    <SelectItem key={server.name} value={server.name}>
                      {server.name}
                      {server.version && ` (v${server.version})`}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {servers.length === 0 && (
                <p className="text-xs text-muted-foreground">
                  No servers defined in registry. You can still enter a server name manually.
                </p>
              )}
              {servers.length === 0 && (
                <Input
                  value={source.server || source.target || ""}
                  onChange={(e) => onChange({ server: e.target.value, target: undefined })}
                  placeholder="Enter server name"
                />
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="tool">Tool Name *</Label>
              <Input
                id="tool"
                value={source.tool}
                onChange={(e) => onChange({ tool: e.target.value })}
                placeholder="backend_tool_name"
              />
              {/* Show available tools for selected server */}
              {source.server && servers.find((s) => s.name === source.server)?.provides && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {servers
                    .find((s) => s.name === source.server)
                    ?.provides.map((p) => (
                      <Badge
                        key={p.tool}
                        variant="outline"
                        className="cursor-pointer hover:bg-secondary"
                        onClick={() => onChange({ tool: p.tool })}
                      >
                        {p.tool}
                      </Badge>
                    ))}
                </div>
              )}
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="serverVersion">Server Version</Label>
            <Input
              id="serverVersion"
              value={source.serverVersion || ""}
              onChange={(e) => onChange({ serverVersion: e.target.value || undefined })}
              placeholder="Optional: specific version (e.g., 1.2.0)"
            />
            <p className="text-xs text-muted-foreground">
              Route to a specific server version. Leave empty for latest.
            </p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Default Values</CardTitle>
          <CardDescription>Values to inject into tool calls (hidden from agents)</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {Object.entries(source.defaults || {}).map(([key, value], index) => (
            <div key={index} className="flex gap-2 items-start">
              <div className="flex-1 space-y-1">
                <Input
                  value={key}
                  onChange={(e) => updateDefault(key, e.target.value, value)}
                  placeholder="Field name"
                />
              </div>
              <div className="flex-1 space-y-1">
                <Input
                  value={typeof value === "string" ? value : JSON.stringify(value)}
                  onChange={(e) => {
                    let newValue: unknown = e.target.value;
                    // Try to parse as JSON
                    try {
                      newValue = JSON.parse(e.target.value);
                    } catch {
                      // Keep as string
                    }
                    updateDefault(key, key, newValue);
                  }}
                  placeholder="Value (supports ${ENV_VAR})"
                />
              </div>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => removeDefault(key)}
                className="text-destructive"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ))}
          <Button variant="outline" size="sm" onClick={addDefault}>
            <Plus className="h-4 w-4 mr-2" />
            Add Default
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Hidden Fields</CardTitle>
          <CardDescription>
            Fields to remove from the tool schema (not shown to agents)
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            {source.hideFields?.map((field) => (
              <Badge key={field} variant="secondary" className="gap-1">
                {field}
                <button
                  onClick={() => removeHideField(field)}
                  className="ml-1 hover:text-destructive"
                >
                  &times;
                </button>
              </Badge>
            ))}
          </div>
          <Input
            placeholder="Add field name and press Enter"
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                const input = e.currentTarget;
                addHideField(input.value.trim());
                input.value = "";
              }
            }}
          />
        </CardContent>
      </Card>
    </div>
  );
}
