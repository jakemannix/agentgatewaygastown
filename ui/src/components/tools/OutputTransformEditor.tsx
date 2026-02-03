"use client";

import { Plus, Trash2 } from "lucide-react";
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
import { OutputTransform, FieldSource, getFieldSourceType } from "@/lib/registry-types";

interface OutputTransformEditorProps {
  transform: OutputTransform | undefined;
  onChange: (transform: OutputTransform | undefined) => void;
}

export function OutputTransformEditor({ transform, onChange }: OutputTransformEditorProps) {
  const ensureTransform = (): OutputTransform => {
    return transform || { mappings: {} };
  };

  const addMapping = () => {
    const t = ensureTransform();
    const newMappings = { ...t.mappings, "": { path: "$." } };
    onChange({ mappings: newMappings });
  };

  const updateMappingKey = (oldKey: string, newKey: string) => {
    const t = ensureTransform();
    const newMappings = { ...t.mappings };
    const value = newMappings[oldKey];
    delete newMappings[oldKey];
    newMappings[newKey] = value;
    onChange({ mappings: newMappings });
  };

  const updateMappingValue = (key: string, source: FieldSource) => {
    const t = ensureTransform();
    onChange({ mappings: { ...t.mappings, [key]: source } });
  };

  const removeMapping = (key: string) => {
    const t = ensureTransform();
    const newMappings = { ...t.mappings };
    delete newMappings[key];
    onChange(Object.keys(newMappings).length > 0 ? { mappings: newMappings } : undefined);
  };

  const renderFieldSourceEditor = (
    source: FieldSource,
    onSourceChange: (source: FieldSource) => void
  ) => {
    const sourceType = getFieldSourceType(source);

    return (
      <div className="space-y-2 flex-1">
        <Select
          value={sourceType}
          onValueChange={(v) => {
            switch (v) {
              case "path":
                onSourceChange({ path: "$." });
                break;
              case "literal":
                onSourceChange({ literal: { stringValue: "" } });
                break;
              case "coalesce":
                onSourceChange({ coalesce: { paths: ["$.", "$."] } });
                break;
              case "template":
                onSourceChange({ template: { template: "", vars: {} } });
                break;
              case "concat":
                onSourceChange({ concat: { paths: [] } });
                break;
              case "arrayMap":
                onSourceChange({ arrayMap: { over: "$.", each: {} } });
                break;
            }
          }}
        >
          <SelectTrigger className="w-[130px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="path">JSONPath</SelectItem>
            <SelectItem value="literal">Literal</SelectItem>
            <SelectItem value="coalesce">Coalesce</SelectItem>
            <SelectItem value="template">Template</SelectItem>
            <SelectItem value="concat">Concat</SelectItem>
            <SelectItem value="arrayMap">Array Map</SelectItem>
          </SelectContent>
        </Select>

        {"path" in source && (
          <Input
            value={source.path}
            onChange={(e) => onSourceChange({ path: e.target.value })}
            placeholder="$.field.path"
          />
        )}

        {"literal" in source && (
          <div className="space-y-2">
            <Select
              value={
                "stringValue" in source.literal
                  ? "string"
                  : "numberValue" in source.literal
                    ? "number"
                    : "boolValue" in source.literal
                      ? "boolean"
                      : "null"
              }
              onValueChange={(v) => {
                switch (v) {
                  case "string":
                    onSourceChange({ literal: { stringValue: "" } });
                    break;
                  case "number":
                    onSourceChange({ literal: { numberValue: 0 } });
                    break;
                  case "boolean":
                    onSourceChange({ literal: { boolValue: false } });
                    break;
                  case "null":
                    onSourceChange({ literal: { nullValue: true } });
                    break;
                }
              }}
            >
              <SelectTrigger className="w-[100px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="string">String</SelectItem>
                <SelectItem value="number">Number</SelectItem>
                <SelectItem value="boolean">Boolean</SelectItem>
                <SelectItem value="null">Null</SelectItem>
              </SelectContent>
            </Select>
            {"stringValue" in source.literal && (
              <Input
                value={source.literal.stringValue}
                onChange={(e) => onSourceChange({ literal: { stringValue: e.target.value } })}
                placeholder="String value"
              />
            )}
            {"numberValue" in source.literal && (
              <Input
                type="number"
                value={source.literal.numberValue}
                onChange={(e) =>
                  onSourceChange({
                    literal: { numberValue: parseFloat(e.target.value) || 0 },
                  })
                }
              />
            )}
            {"boolValue" in source.literal && (
              <Select
                value={source.literal.boolValue ? "true" : "false"}
                onValueChange={(v) => onSourceChange({ literal: { boolValue: v === "true" } })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="true">true</SelectItem>
                  <SelectItem value="false">false</SelectItem>
                </SelectContent>
              </Select>
            )}
          </div>
        )}

        {"coalesce" in source && (
          <div className="space-y-2">
            {source.coalesce.paths.map((path, i) => (
              <div key={i} className="flex gap-2">
                <Input
                  value={path}
                  onChange={(e) => {
                    const newPaths = [...source.coalesce.paths];
                    newPaths[i] = e.target.value;
                    onSourceChange({ coalesce: { paths: newPaths } });
                  }}
                  placeholder={`Path ${i + 1}`}
                />
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => {
                    const newPaths = source.coalesce.paths.filter((_, idx) => idx !== i);
                    onSourceChange({ coalesce: { paths: newPaths } });
                  }}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            ))}
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                onSourceChange({
                  coalesce: { paths: [...source.coalesce.paths, "$."] },
                })
              }
            >
              <Plus className="h-4 w-4 mr-1" /> Add Path
            </Button>
          </div>
        )}

        {"template" in source && (
          <div className="space-y-2">
            <Input
              value={source.template.template}
              onChange={(e) =>
                onSourceChange({
                  template: { ...source.template, template: e.target.value },
                })
              }
              placeholder="Template: {var1} - {var2}"
            />
            <p className="text-xs text-muted-foreground">Variables (key = JSONPath):</p>
            <pre className="text-xs bg-muted p-2 rounded">
              {JSON.stringify(source.template.vars, null, 2)}
            </pre>
          </div>
        )}

        {"arrayMap" in source && (
          <div className="space-y-2">
            <div className="space-y-1">
              <Label className="text-xs">Array Path (over)</Label>
              <Input
                value={source.arrayMap.over}
                onChange={(e) =>
                  onSourceChange({
                    arrayMap: { ...source.arrayMap, over: e.target.value },
                  })
                }
                placeholder="$.results"
              />
            </div>
            <div className="space-y-1">
              <Label className="text-xs">Element Mappings (each)</Label>
              <p className="text-xs text-muted-foreground">
                Edit in JSON mode for nested arrayMap mappings
              </p>
              <pre className="text-xs bg-muted p-2 rounded max-h-[150px] overflow-auto">
                {JSON.stringify(source.arrayMap.each, null, 2)}
              </pre>
            </div>
          </div>
        )}
      </div>
    );
  };

  const mappings = transform?.mappings || {};
  const hasTransform = Object.keys(mappings).length > 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Output Transform</CardTitle>
        <CardDescription>
          Transform the backend tool output to a different schema. Use JSONPath expressions to
          extract and remap fields.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {!hasTransform && (
          <div className="text-center py-6 text-muted-foreground">
            <p className="mb-4">No output transform configured. Output passes through unchanged.</p>
            <Button variant="outline" onClick={addMapping}>
              <Plus className="h-4 w-4 mr-2" />
              Add Transform Mapping
            </Button>
          </div>
        )}

        {hasTransform && (
          <>
            {Object.entries(mappings).map(([key, source]) => (
              <div key={key} className="flex gap-4 items-start p-3 border rounded-lg">
                <div className="space-y-1 w-[200px]">
                  <Label className="text-xs">Output Field</Label>
                  <Input
                    value={key}
                    onChange={(e) => updateMappingKey(key, e.target.value)}
                    placeholder="fieldName"
                  />
                </div>
                <div className="space-y-1 flex-1">
                  <Label className="text-xs">Source</Label>
                  {renderFieldSourceEditor(source, (newSource) =>
                    updateMappingValue(key, newSource)
                  )}
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => removeMapping(key)}
                  className="text-destructive mt-6"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            ))}
            <Button variant="outline" size="sm" onClick={addMapping}>
              <Plus className="h-4 w-4 mr-2" />
              Add Mapping
            </Button>
          </>
        )}
      </CardContent>
    </Card>
  );
}
