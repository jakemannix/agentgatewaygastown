"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { ArrowLeft, Save, Play, Plus, Trash2, ChevronDown, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
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
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { toast } from "sonner";
import { fetchRegistry, saveRegistryTool } from "@/lib/api";
import {
  Registry,
  ToolDefinition,
  SourceTool,
  ScatterGatherSpec,
  PipelineSpec,
  AggregationOp,
  ScatterTarget,
  PipelineStep,
  FieldSource,
  OutputTransform,
  JsonSchema,
  createEmptySourceTool,
  createEmptyScatterGatherTool,
  createEmptyPipelineTool,
} from "@/lib/registry-types";
import { SourceToolEditor } from "./SourceToolEditor";
import { ScatterGatherEditor } from "./ScatterGatherEditor";
import { PipelineEditor } from "./PipelineEditor";
import { OutputTransformEditor } from "./OutputTransformEditor";
import { SchemaEditor } from "./SchemaEditor";

type ToolType = "source" | "scatterGather" | "pipeline";

interface ToolEditorProps {
  toolName?: string; // If provided, edit mode; otherwise create mode
}

export function ToolEditor({ toolName }: ToolEditorProps) {
  const router = useRouter();
  const [registry, setRegistry] = useState<Registry | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [tool, setTool] = useState<ToolDefinition | null>(null);
  const [toolType, setToolType] = useState<ToolType>("source");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [jsonMode, setJsonMode] = useState(false);
  const [jsonContent, setJsonContent] = useState("");
  const [jsonError, setJsonError] = useState<string | null>(null);

  const isEditMode = !!toolName;

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      const reg = await fetchRegistry();
      setRegistry(reg);

      if (toolName) {
        // Edit mode - find the tool
        const existingTool = reg.tools?.find((t) => t.name === toolName);
        if (existingTool) {
          setTool(existingTool);
          // Determine tool type
          if (existingTool.source) {
            setToolType("source");
          } else if (existingTool.spec && "scatterGather" in existingTool.spec) {
            setToolType("scatterGather");
          } else if (existingTool.spec && "pipeline" in existingTool.spec) {
            setToolType("pipeline");
          }
          setJsonContent(JSON.stringify(existingTool, null, 2));
        } else {
          toast.error(`Tool "${toolName}" not found`);
          router.push("/tools");
        }
      } else {
        // Create mode - initialize empty tool
        setTool(createEmptySourceTool("new_tool"));
        setJsonContent(JSON.stringify(createEmptySourceTool("new_tool"), null, 2));
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to load registry");
      console.error(err);
    } finally {
      setLoading(false);
    }
  }, [toolName, router]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Sync JSON content when tool changes
  useEffect(() => {
    if (tool && !jsonMode) {
      setJsonContent(JSON.stringify(tool, null, 2));
    }
  }, [tool, jsonMode]);

  const handleToolTypeChange = (newType: ToolType) => {
    if (!tool) return;

    let newTool: ToolDefinition;
    switch (newType) {
      case "source":
        newTool = {
          ...tool,
          source: { server: "", tool: "" },
          spec: undefined,
        };
        break;
      case "scatterGather":
        newTool = {
          ...tool,
          source: undefined,
          spec: {
            scatterGather: {
              targets: [],
              aggregation: { ops: [{ flatten: true }] },
            },
          },
        };
        break;
      case "pipeline":
        newTool = {
          ...tool,
          source: undefined,
          spec: {
            pipeline: { steps: [] },
          },
        };
        break;
    }

    setToolType(newType);
    setTool(newTool);
  };

  const handleSave = async () => {
    if (!tool) return;

    // Validate tool name
    if (!tool.name || tool.name.trim() === "") {
      toast.error("Tool name is required");
      return;
    }

    // Validate based on type
    if (toolType === "source" && (!tool.source?.server || !tool.source?.tool)) {
      toast.error("Source tool requires server and tool name");
      return;
    }

    try {
      setSaving(true);
      await saveRegistryTool(tool);
      toast.success(`Tool "${tool.name}" saved successfully`);
      router.push("/tools");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save tool");
    } finally {
      setSaving(false);
    }
  };

  const handleJsonSave = () => {
    try {
      const parsed = JSON.parse(jsonContent);
      setTool(parsed);
      setJsonError(null);
      setJsonMode(false);
      toast.success("JSON applied successfully");
    } catch (err) {
      setJsonError(err instanceof Error ? err.message : "Invalid JSON");
    }
  };

  const updateTool = (updates: Partial<ToolDefinition>) => {
    if (!tool) return;
    setTool({ ...tool, ...updates });
  };

  const updateSourceTool = (updates: Partial<SourceTool>) => {
    if (!tool?.source) return;
    setTool({
      ...tool,
      source: { ...tool.source, ...updates },
    });
  };

  const updateScatterGather = (updates: Partial<ScatterGatherSpec>) => {
    if (!tool?.spec || !("scatterGather" in tool.spec)) return;
    setTool({
      ...tool,
      spec: {
        scatterGather: { ...tool.spec.scatterGather, ...updates },
      },
    });
  };

  const updatePipeline = (updates: Partial<PipelineSpec>) => {
    if (!tool?.spec || !("pipeline" in tool.spec)) return;
    setTool({
      ...tool,
      spec: {
        pipeline: { ...tool.spec.pipeline, ...updates },
      },
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Loading...</div>
      </div>
    );
  }

  if (!tool) {
    return (
      <div className="p-6">
        <div className="text-destructive">Tool not found</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="icon" onClick={() => router.push("/tools")}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <div>
            <h1 className="text-2xl font-bold">
              {isEditMode ? `Edit: ${tool.name}` : "Create Tool"}
            </h1>
            <p className="text-muted-foreground">
              {isEditMode ? "Modify virtual tool configuration" : "Define a new virtual tool"}
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => setJsonMode(!jsonMode)}>
            {jsonMode ? "Visual Editor" : "JSON Editor"}
          </Button>
          <Button onClick={handleSave} disabled={saving}>
            <Save className="h-4 w-4 mr-2" />
            {saving ? "Saving..." : "Save"}
          </Button>
        </div>
      </div>

      {jsonMode ? (
        /* JSON Editor Mode */
        <Card>
          <CardHeader>
            <CardTitle>JSON Editor</CardTitle>
            <CardDescription>Edit the tool definition directly as JSON</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Textarea
              value={jsonContent}
              onChange={(e) => {
                setJsonContent(e.target.value);
                setJsonError(null);
              }}
              className="font-mono text-sm min-h-[400px]"
              placeholder="Paste tool JSON here..."
            />
            {jsonError && <div className="text-sm text-destructive">{jsonError}</div>}
            <Button onClick={handleJsonSave}>Apply JSON</Button>
          </CardContent>
        </Card>
      ) : (
        /* Visual Editor Mode */
        <Tabs defaultValue="basic" className="space-y-4">
          <TabsList>
            <TabsTrigger value="basic">Basic Info</TabsTrigger>
            <TabsTrigger value="implementation">Implementation</TabsTrigger>
            <TabsTrigger value="transform">Transform</TabsTrigger>
            <TabsTrigger value="schema">Schema</TabsTrigger>
          </TabsList>

          {/* Basic Info Tab */}
          <TabsContent value="basic" className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>Tool Information</CardTitle>
                <CardDescription>Basic tool metadata and identification</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="name">Name *</Label>
                    <Input
                      id="name"
                      value={tool.name}
                      onChange={(e) => updateTool({ name: e.target.value })}
                      placeholder="my_virtual_tool"
                      disabled={isEditMode}
                    />
                    {isEditMode && (
                      <p className="text-xs text-muted-foreground">
                        Tool name cannot be changed after creation
                      </p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="version">Version</Label>
                    <Input
                      id="version"
                      value={tool.version || ""}
                      onChange={(e) => updateTool({ version: e.target.value || undefined })}
                      placeholder="1.0.0"
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="description">Description</Label>
                  <Textarea
                    id="description"
                    value={tool.description || ""}
                    onChange={(e) => updateTool({ description: e.target.value || undefined })}
                    placeholder="Describe what this tool does..."
                    rows={3}
                  />
                </div>

                <div className="space-y-2">
                  <Label>Tags</Label>
                  <div className="flex flex-wrap gap-2 mb-2">
                    {tool.tags?.map((tag, index) => (
                      <Badge key={index} variant="secondary" className="gap-1">
                        {tag}
                        <button
                          onClick={() => {
                            const newTags = tool.tags?.filter((_, i) => i !== index);
                            updateTool({ tags: newTags });
                          }}
                          className="ml-1 hover:text-destructive"
                        >
                          &times;
                        </button>
                      </Badge>
                    ))}
                  </div>
                  <Input
                    placeholder="Add tag and press Enter"
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        const input = e.currentTarget;
                        const value = input.value.trim();
                        if (value && !tool.tags?.includes(value)) {
                          updateTool({ tags: [...(tool.tags || []), value] });
                          input.value = "";
                        }
                      }
                    }}
                  />
                </div>
              </CardContent>
            </Card>

            {/* Tool Type Selection */}
            <Card>
              <CardHeader>
                <CardTitle>Tool Type</CardTitle>
                <CardDescription>Choose how this tool is implemented</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid gap-4 md:grid-cols-3">
                  <Card
                    className={`cursor-pointer transition-all ${
                      toolType === "source"
                        ? "border-primary ring-2 ring-primary"
                        : "hover:border-primary"
                    }`}
                    onClick={() => handleToolTypeChange("source")}
                  >
                    <CardHeader className="pb-2">
                      <CardTitle className="text-sm">Source Tool</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <p className="text-xs text-muted-foreground">
                        Maps to a single backend tool with optional transformation
                      </p>
                    </CardContent>
                  </Card>

                  <Card
                    className={`cursor-pointer transition-all ${
                      toolType === "scatterGather"
                        ? "border-primary ring-2 ring-primary"
                        : "hover:border-primary"
                    }`}
                    onClick={() => handleToolTypeChange("scatterGather")}
                  >
                    <CardHeader className="pb-2">
                      <CardTitle className="text-sm">Scatter-Gather</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <p className="text-xs text-muted-foreground">
                        Fan-out to multiple tools in parallel and aggregate results
                      </p>
                    </CardContent>
                  </Card>

                  <Card
                    className={`cursor-pointer transition-all ${
                      toolType === "pipeline"
                        ? "border-primary ring-2 ring-primary"
                        : "hover:border-primary"
                    }`}
                    onClick={() => handleToolTypeChange("pipeline")}
                  >
                    <CardHeader className="pb-2">
                      <CardTitle className="text-sm">Pipeline</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <p className="text-xs text-muted-foreground">
                        Execute tools sequentially, passing data between steps
                      </p>
                    </CardContent>
                  </Card>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          {/* Implementation Tab */}
          <TabsContent value="implementation" className="space-y-4">
            {toolType === "source" && tool.source && (
              <SourceToolEditor
                source={tool.source}
                servers={registry?.servers || []}
                onChange={updateSourceTool}
              />
            )}

            {toolType === "scatterGather" && tool.spec && "scatterGather" in tool.spec && (
              <ScatterGatherEditor
                spec={tool.spec.scatterGather}
                availableTools={registry?.tools?.map((t) => t.name) || []}
                onChange={updateScatterGather}
              />
            )}

            {toolType === "pipeline" && tool.spec && "pipeline" in tool.spec && (
              <PipelineEditor
                spec={tool.spec.pipeline}
                availableTools={registry?.tools?.map((t) => t.name) || []}
                servers={registry?.servers || []}
                onChange={updatePipeline}
              />
            )}
          </TabsContent>

          {/* Transform Tab */}
          <TabsContent value="transform" className="space-y-4">
            <OutputTransformEditor
              transform={tool.outputTransform}
              onChange={(transform) => updateTool({ outputTransform: transform })}
            />
          </TabsContent>

          {/* Schema Tab */}
          <TabsContent value="schema" className="space-y-4">
            <SchemaEditor
              inputSchema={tool.inputSchema}
              outputSchema={tool.outputSchema as JsonSchema | undefined}
              availableSchemas={registry?.schemas || []}
              onInputChange={(schema) => updateTool({ inputSchema: schema })}
              onOutputChange={(schema) => updateTool({ outputSchema: schema })}
            />
          </TabsContent>
        </Tabs>
      )}

      {/* Advanced Settings */}
      <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
        <CollapsibleTrigger asChild>
          <Button variant="ghost" className="w-full justify-between">
            Advanced Settings
            {advancedOpen ? (
              <ChevronDown className="h-4 w-4" />
            ) : (
              <ChevronRight className="h-4 w-4" />
            )}
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-4 pt-4">
          <Card>
            <CardHeader>
              <CardTitle>Deprecation</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <Label htmlFor="deprecated">Deprecation Message</Label>
                <Input
                  id="deprecated"
                  value={tool.deprecated || ""}
                  onChange={(e) => updateTool({ deprecated: e.target.value || undefined })}
                  placeholder="Optional: Migration message if deprecated"
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Metadata</CardTitle>
              <CardDescription>Custom key-value pairs for this tool</CardDescription>
            </CardHeader>
            <CardContent>
              <pre className="text-xs bg-muted p-2 rounded">
                {JSON.stringify(tool.metadata || {}, null, 2)}
              </pre>
            </CardContent>
          </Card>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
