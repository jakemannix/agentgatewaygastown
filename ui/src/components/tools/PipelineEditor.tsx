"use client";

import { Plus, Trash2, GripVertical, ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  PipelineSpec,
  PipelineStep,
  StepOperation,
  DataBinding,
  Server,
} from "@/lib/registry-types";

interface PipelineEditorProps {
  spec: PipelineSpec;
  availableTools: string[];
  servers: Server[];
  onChange: (updates: Partial<PipelineSpec>) => void;
}

export function PipelineEditor({ spec, availableTools, servers, onChange }: PipelineEditorProps) {
  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set());

  const toggleStep = (id: string) => {
    const newExpanded = new Set(expandedSteps);
    if (newExpanded.has(id)) {
      newExpanded.delete(id);
    } else {
      newExpanded.add(id);
    }
    setExpandedSteps(newExpanded);
  };

  const addStep = () => {
    const newId = `step_${spec.steps.length + 1}`;
    const newStep: PipelineStep = {
      id: newId,
      operation: { tool: { name: "" } },
    };
    onChange({ steps: [...spec.steps, newStep] });
    setExpandedSteps(new Set([...expandedSteps, newId]));
  };

  const updateStep = (index: number, updates: Partial<PipelineStep>) => {
    const newSteps = [...spec.steps];
    newSteps[index] = { ...newSteps[index], ...updates };
    onChange({ steps: newSteps });
  };

  const removeStep = (index: number) => {
    const newSteps = spec.steps.filter((_, i) => i !== index);
    onChange({ steps: newSteps });
  };

  const moveStep = (index: number, direction: "up" | "down") => {
    const newSteps = [...spec.steps];
    const targetIndex = direction === "up" ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= newSteps.length) return;
    [newSteps[index], newSteps[targetIndex]] = [newSteps[targetIndex], newSteps[index]];
    onChange({ steps: newSteps });
  };

  const getOperationType = (op: StepOperation): "tool" | "agent" | "pattern" => {
    if ("tool" in op) return "tool";
    if ("agent" in op) return "agent";
    return "pattern";
  };

  const renderDataBindingEditor = (
    binding: DataBinding | undefined,
    onBindingChange: (binding: DataBinding | undefined) => void,
    stepIds: string[]
  ) => {
    const getBindingType = (b: DataBinding | undefined): string => {
      if (!b) return "none";
      if ("input" in b) return "input";
      if ("step" in b) return "step";
      if ("constant" in b) return "constant";
      if ("construct" in b) return "construct";
      return "none";
    };

    const bindingType = getBindingType(binding);

    return (
      <div className="space-y-2">
        <Select
          value={bindingType}
          onValueChange={(v) => {
            if (v === "none") {
              onBindingChange(undefined);
            } else if (v === "input") {
              onBindingChange({ input: { path: "$" } });
            } else if (v === "step") {
              onBindingChange({ step: { stepId: stepIds[0] || "", path: "$" } });
            } else if (v === "constant") {
              onBindingChange({ constant: {} });
            } else if (v === "construct") {
              onBindingChange({ construct: { fields: {} } });
            }
          }}
        >
          <SelectTrigger className="w-[150px]">
            <SelectValue placeholder="Binding type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">No binding</SelectItem>
            <SelectItem value="input">From Input</SelectItem>
            <SelectItem value="step">From Step</SelectItem>
            <SelectItem value="constant">Constant</SelectItem>
            <SelectItem value="construct">Construct</SelectItem>
          </SelectContent>
        </Select>

        {binding && "input" in binding && (
          <Input
            value={binding.input.path}
            onChange={(e) => onBindingChange({ input: { path: e.target.value } })}
            placeholder="JSONPath (e.g., $.query)"
          />
        )}

        {binding && "step" in binding && (
          <div className="flex gap-2">
            <Select
              value={binding.step.stepId}
              onValueChange={(v) => onBindingChange({ step: { ...binding.step, stepId: v } })}
            >
              <SelectTrigger className="w-[150px]">
                <SelectValue placeholder="From step" />
              </SelectTrigger>
              <SelectContent>
                {stepIds.map((id) => (
                  <SelectItem key={id} value={id}>
                    {id}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Input
              value={binding.step.path}
              onChange={(e) => onBindingChange({ step: { ...binding.step, path: e.target.value } })}
              placeholder="JSONPath"
              className="flex-1"
            />
          </div>
        )}

        {binding && "constant" in binding && (
          <Textarea
            value={JSON.stringify(binding.constant, null, 2)}
            onChange={(e) => {
              try {
                const parsed = JSON.parse(e.target.value);
                onBindingChange({ constant: parsed });
              } catch {
                // Invalid JSON, keep current value
              }
            }}
            placeholder='{"key": "value"}'
            className="font-mono text-sm"
            rows={3}
          />
        )}

        {binding && "construct" in binding && (
          <div className="space-y-2 pl-4 border-l-2">
            <p className="text-xs text-muted-foreground">
              Construct object from multiple sources. Edit in JSON mode for full control.
            </p>
            <pre className="text-xs bg-muted p-2 rounded">
              {JSON.stringify(binding.construct, null, 2)}
            </pre>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>Pipeline Steps</CardTitle>
          <CardDescription>
            Steps execute sequentially. Each step can use data from previous steps.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          {spec.steps.map((step, index) => {
            const isExpanded = expandedSteps.has(step.id);
            const opType = getOperationType(step.operation);
            const previousStepIds = spec.steps.slice(0, index).map((s) => s.id);

            return (
              <Collapsible key={step.id} open={isExpanded} onOpenChange={() => toggleStep(step.id)}>
                <div className="border rounded-lg">
                  <CollapsibleTrigger asChild>
                    <div className="flex items-center gap-2 p-3 cursor-pointer hover:bg-muted/50">
                      <GripVertical className="h-4 w-4 text-muted-foreground" />
                      {isExpanded ? (
                        <ChevronDown className="h-4 w-4" />
                      ) : (
                        <ChevronRight className="h-4 w-4" />
                      )}
                      <span className="font-medium">{step.id}</span>
                      <span className="text-muted-foreground text-sm">
                        {opType === "tool" && "tool" in step.operation
                          ? `Tool: ${step.operation.tool.name || "(not set)"}`
                          : opType}
                      </span>
                      <div className="flex-1" />
                      <div className="flex gap-1">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={(e) => {
                            e.stopPropagation();
                            moveStep(index, "up");
                          }}
                          disabled={index === 0}
                        >
                          Up
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={(e) => {
                            e.stopPropagation();
                            moveStep(index, "down");
                          }}
                          disabled={index === spec.steps.length - 1}
                        >
                          Down
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={(e) => {
                            e.stopPropagation();
                            removeStep(index);
                          }}
                          className="text-destructive"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </div>
                  </CollapsibleTrigger>

                  <CollapsibleContent>
                    <div className="p-4 pt-0 space-y-4 border-t">
                      <div className="grid gap-4 md:grid-cols-2">
                        <div className="space-y-2">
                          <Label>Step ID</Label>
                          <Input
                            value={step.id}
                            onChange={(e) => updateStep(index, { id: e.target.value })}
                            placeholder="unique_step_id"
                          />
                        </div>
                        <div className="space-y-2">
                          <Label>Operation Type</Label>
                          <Select
                            value={opType}
                            onValueChange={(v) => {
                              if (v === "tool") {
                                updateStep(index, { operation: { tool: { name: "" } } });
                              } else if (v === "agent") {
                                updateStep(index, { operation: { agent: { name: "" } } });
                              }
                            }}
                          >
                            <SelectTrigger>
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="tool">Tool</SelectItem>
                              <SelectItem value="agent">Agent</SelectItem>
                              <SelectItem value="pattern">Pattern (use JSON)</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>
                      </div>

                      {opType === "tool" && "tool" in step.operation && (
                        <div className="grid gap-4 md:grid-cols-2">
                          <div className="space-y-2">
                            <Label>Tool Name</Label>
                            <Select
                              value={step.operation.tool.name}
                              onValueChange={(v) =>
                                updateStep(index, {
                                  operation: {
                                    tool: { ...step.operation.tool, name: v },
                                  },
                                })
                              }
                            >
                              <SelectTrigger>
                                <SelectValue placeholder="Select tool" />
                              </SelectTrigger>
                              <SelectContent>
                                {availableTools.map((tool) => (
                                  <SelectItem key={tool} value={tool}>
                                    {tool}
                                  </SelectItem>
                                ))}
                                {/* Also show server tools */}
                                {servers.flatMap((s) =>
                                  s.provides.map((p) => (
                                    <SelectItem key={`${s.name}/${p.tool}`} value={p.tool}>
                                      {s.name}/{p.tool}
                                    </SelectItem>
                                  ))
                                )}
                              </SelectContent>
                            </Select>
                          </div>
                          <div className="space-y-2">
                            <Label>Server (optional)</Label>
                            <Select
                              value={step.operation.tool.server || ""}
                              onValueChange={(v) =>
                                updateStep(index, {
                                  operation: {
                                    tool: {
                                      ...step.operation.tool,
                                      server: v || undefined,
                                    },
                                  },
                                })
                              }
                            >
                              <SelectTrigger>
                                <SelectValue placeholder="Auto-resolve" />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="">Auto-resolve</SelectItem>
                                {servers.map((s) => (
                                  <SelectItem key={s.name} value={s.name}>
                                    {s.name}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          </div>
                        </div>
                      )}

                      {opType === "agent" && "agent" in step.operation && (
                        <div className="grid gap-4 md:grid-cols-2">
                          <div className="space-y-2">
                            <Label>Agent Name</Label>
                            <Input
                              value={step.operation.agent.name}
                              onChange={(e) =>
                                updateStep(index, {
                                  operation: {
                                    agent: { ...step.operation.agent, name: e.target.value },
                                  },
                                })
                              }
                              placeholder="agent_name"
                            />
                          </div>
                          <div className="space-y-2">
                            <Label>Skill (optional)</Label>
                            <Input
                              value={step.operation.agent.skill || ""}
                              onChange={(e) =>
                                updateStep(index, {
                                  operation: {
                                    agent: {
                                      ...step.operation.agent,
                                      skill: e.target.value || undefined,
                                    },
                                  },
                                })
                              }
                              placeholder="specific_skill"
                            />
                          </div>
                        </div>
                      )}

                      <div className="space-y-2">
                        <Label>Input Binding</Label>
                        <p className="text-xs text-muted-foreground">
                          How to get input for this step (from original input or previous step)
                        </p>
                        {renderDataBindingEditor(
                          step.input,
                          (binding) => updateStep(index, { input: binding }),
                          previousStepIds
                        )}
                      </div>
                    </div>
                  </CollapsibleContent>
                </div>
              </Collapsible>
            );
          })}

          <Button variant="outline" onClick={addStep} className="w-full">
            <Plus className="h-4 w-4 mr-2" />
            Add Step
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
