"use client";

import { Plus, Trash2, GripVertical } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  ScatterGatherSpec,
  ScatterTarget,
  AggregationOp,
  AggregationStrategy,
} from "@/lib/registry-types";

interface ScatterGatherEditorProps {
  spec: ScatterGatherSpec;
  availableTools: string[];
  onChange: (updates: Partial<ScatterGatherSpec>) => void;
}

export function ScatterGatherEditor({ spec, availableTools, onChange }: ScatterGatherEditorProps) {
  const addTarget = () => {
    const newTargets: ScatterTarget[] = [...spec.targets, { tool: "" }];
    onChange({ targets: newTargets });
  };

  const updateTarget = (index: number, toolName: string, server?: string) => {
    const newTargets = [...spec.targets];
    newTargets[index] = server ? { tool: toolName, server } : { tool: toolName };
    onChange({ targets: newTargets });
  };

  const removeTarget = (index: number) => {
    const newTargets = spec.targets.filter((_, i) => i !== index);
    onChange({ targets: newTargets });
  };

  const addAggregationOp = (op: AggregationOp) => {
    const newOps = [...spec.aggregation.ops, op];
    onChange({ aggregation: { ops: newOps } });
  };

  const updateAggregationOp = (index: number, op: AggregationOp) => {
    const newOps = [...spec.aggregation.ops];
    newOps[index] = op;
    onChange({ aggregation: { ops: newOps } });
  };

  const removeAggregationOp = (index: number) => {
    const newOps = spec.aggregation.ops.filter((_, i) => i !== index);
    onChange({ aggregation: { ops: newOps } });
  };

  const getOpType = (op: AggregationOp): string => {
    if ("flatten" in op) return "flatten";
    if ("sort" in op) return "sort";
    if ("dedupe" in op) return "dedupe";
    if ("limit" in op) return "limit";
    if ("concat" in op) return "concat";
    if ("merge" in op) return "merge";
    if ("wrap" in op) return "wrap";
    if ("extract" in op) return "extract";
    return "unknown";
  };

  const renderOpEditor = (op: AggregationOp, index: number) => {
    const opType = getOpType(op);

    return (
      <div className="flex gap-2 items-center p-2 border rounded bg-muted/50">
        <GripVertical className="h-4 w-4 text-muted-foreground cursor-move" />
        <Select
          value={opType}
          onValueChange={(newType) => {
            let newOp: AggregationOp;
            switch (newType) {
              case "flatten":
                newOp = { flatten: true };
                break;
              case "sort":
                newOp = { sort: { field: "$.score", order: "desc" } };
                break;
              case "dedupe":
                newOp = { dedupe: { field: "$.url" } };
                break;
              case "limit":
                newOp = { limit: { count: 10 } };
                break;
              case "concat":
                newOp = { concat: true };
                break;
              case "merge":
                newOp = { merge: true };
                break;
              case "wrap":
                newOp = { wrap: { field: "results" } };
                break;
              case "extract":
                newOp = { extract: { path: "$.results" } };
                break;
              default:
                return;
            }
            updateAggregationOp(index, newOp);
          }}
        >
          <SelectTrigger className="w-[120px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="flatten">Flatten</SelectItem>
            <SelectItem value="sort">Sort</SelectItem>
            <SelectItem value="dedupe">Dedupe</SelectItem>
            <SelectItem value="limit">Limit</SelectItem>
            <SelectItem value="concat">Concat</SelectItem>
            <SelectItem value="merge">Merge</SelectItem>
            <SelectItem value="wrap">Wrap</SelectItem>
            <SelectItem value="extract">Extract</SelectItem>
          </SelectContent>
        </Select>

        {/* Op-specific fields */}
        {"sort" in op && (
          <>
            <Input
              value={op.sort.field}
              onChange={(e) =>
                updateAggregationOp(index, { sort: { ...op.sort, field: e.target.value } })
              }
              placeholder="$.field"
              className="w-[150px]"
            />
            <Select
              value={op.sort.order}
              onValueChange={(v) =>
                updateAggregationOp(index, { sort: { ...op.sort, order: v as "asc" | "desc" } })
              }
            >
              <SelectTrigger className="w-[80px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="asc">Asc</SelectItem>
                <SelectItem value="desc">Desc</SelectItem>
              </SelectContent>
            </Select>
          </>
        )}

        {"dedupe" in op && (
          <Input
            value={op.dedupe.field}
            onChange={(e) => updateAggregationOp(index, { dedupe: { field: e.target.value } })}
            placeholder="$.url"
            className="flex-1"
          />
        )}

        {"limit" in op && (
          <Input
            type="number"
            value={op.limit.count}
            onChange={(e) =>
              updateAggregationOp(index, { limit: { count: parseInt(e.target.value) || 10 } })
            }
            className="w-[80px]"
          />
        )}

        {"wrap" in op && (
          <Input
            value={op.wrap.field}
            onChange={(e) => updateAggregationOp(index, { wrap: { field: e.target.value } })}
            placeholder="results"
            className="flex-1"
          />
        )}

        {"extract" in op && (
          <Input
            value={op.extract.path}
            onChange={(e) => updateAggregationOp(index, { extract: { path: e.target.value } })}
            placeholder="$.results"
            className="flex-1"
          />
        )}

        <Button
          variant="ghost"
          size="icon"
          onClick={() => removeAggregationOp(index)}
          className="text-destructive"
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
    );
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>Parallel Targets</CardTitle>
          <CardDescription>
            Tools to invoke in parallel. All targets receive the same input.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {spec.targets.map((target, index) => (
            <div key={index} className="flex gap-2 items-center">
              <div className="flex-1">
                <Select
                  value={"tool" in target ? target.tool : ""}
                  onValueChange={(v) => updateTarget(index, v)}
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
                  </SelectContent>
                </Select>
              </div>
              <Input
                value={"tool" in target ? target.server || "" : ""}
                onChange={(e) =>
                  updateTarget(
                    index,
                    "tool" in target ? target.tool : "",
                    e.target.value || undefined
                  )
                }
                placeholder="Optional: server"
                className="w-[150px]"
              />
              <Button
                variant="ghost"
                size="icon"
                onClick={() => removeTarget(index)}
                className="text-destructive"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </div>
          ))}
          <Button variant="outline" size="sm" onClick={addTarget}>
            <Plus className="h-4 w-4 mr-2" />
            Add Target
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Aggregation Pipeline</CardTitle>
          <CardDescription>
            Operations to combine results. Applied in order from top to bottom.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          {spec.aggregation.ops.map((op, index) => (
            <div key={index}>{renderOpEditor(op, index)}</div>
          ))}
          <div className="flex gap-2 pt-2">
            <Select
              onValueChange={(v) => {
                let newOp: AggregationOp;
                switch (v) {
                  case "flatten":
                    newOp = { flatten: true };
                    break;
                  case "sort":
                    newOp = { sort: { field: "$.score", order: "desc" } };
                    break;
                  case "dedupe":
                    newOp = { dedupe: { field: "$.url" } };
                    break;
                  case "limit":
                    newOp = { limit: { count: 10 } };
                    break;
                  case "wrap":
                    newOp = { wrap: { field: "results" } };
                    break;
                  case "extract":
                    newOp = { extract: { path: "$.results" } };
                    break;
                  default:
                    return;
                }
                addAggregationOp(newOp);
              }}
            >
              <SelectTrigger className="w-[200px]">
                <Plus className="h-4 w-4 mr-2" />
                <SelectValue placeholder="Add operation" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="extract">Extract (JSONPath)</SelectItem>
                <SelectItem value="flatten">Flatten Arrays</SelectItem>
                <SelectItem value="dedupe">Deduplicate</SelectItem>
                <SelectItem value="sort">Sort</SelectItem>
                <SelectItem value="limit">Limit Results</SelectItem>
                <SelectItem value="wrap">Wrap in Object</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Execution Options</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <Label>Timeout (ms)</Label>
              <p className="text-xs text-muted-foreground">Maximum time to wait for all targets</p>
            </div>
            <Input
              type="number"
              value={spec.timeoutMs || ""}
              onChange={(e) =>
                onChange({ timeoutMs: e.target.value ? parseInt(e.target.value) : undefined })
              }
              placeholder="30000"
              className="w-[120px]"
            />
          </div>
          <div className="flex items-center justify-between">
            <div>
              <Label>Fail Fast</Label>
              <p className="text-xs text-muted-foreground">Stop immediately if any target fails</p>
            </div>
            <Switch
              checked={spec.failFast}
              onCheckedChange={(checked) => onChange({ failFast: checked })}
            />
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
