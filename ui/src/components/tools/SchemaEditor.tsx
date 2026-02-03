"use client";

import { useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { JsonSchema, Schema } from "@/lib/registry-types";

interface SchemaFormProps {
  schema: JsonSchema | undefined;
  availableSchemas: Schema[];
  onChange: (schema: JsonSchema | undefined) => void;
  jsonError: string | null;
  setJsonError: (error: string | null) => void;
}

function SchemaForm({
  schema,
  availableSchemas,
  onChange,
  jsonError,
  setJsonError,
}: SchemaFormProps) {
  const [jsonMode, setJsonMode] = useState(false);
  const [jsonText, setJsonText] = useState(schema ? JSON.stringify(schema, null, 2) : "");

  // Check if using a $ref
  const isRef = schema && "$ref" in schema && schema.$ref;

  if (jsonMode) {
    return (
      <div className="space-y-2">
        <Textarea
          value={jsonText}
          onChange={(e) => {
            setJsonText(e.target.value);
            setJsonError(null);
          }}
          className="font-mono text-sm min-h-[200px]"
          placeholder='{"type": "object", "properties": {...}}'
        />
        {jsonError && <p className="text-sm text-destructive">{jsonError}</p>}
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              try {
                const parsed = JSON.parse(jsonText);
                onChange(parsed);
                setJsonError(null);
                setJsonMode(false);
              } catch (e) {
                setJsonError(e instanceof Error ? e.message : "Invalid JSON");
              }
            }}
          >
            Apply JSON
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              setJsonText(schema ? JSON.stringify(schema, null, 2) : "");
              setJsonError(null);
              setJsonMode(false);
            }}
          >
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        <Button
          variant={isRef ? "default" : "outline"}
          size="sm"
          onClick={() => {
            if (!isRef && availableSchemas.length > 0) {
              onChange({ $ref: `#/schemas/${availableSchemas[0].name}` });
            }
          }}
        >
          Reference Schema
        </Button>
        <Button
          variant={!isRef && schema ? "default" : "outline"}
          size="sm"
          onClick={() => {
            onChange({
              type: "object",
              properties: {},
            });
          }}
        >
          Inline Schema
        </Button>
        <Button variant="outline" size="sm" onClick={() => setJsonMode(true)}>
          JSON Editor
        </Button>
        {schema && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onChange(undefined)}
            className="text-destructive"
          >
            Clear
          </Button>
        )}
      </div>

      {isRef && (
        <div className="space-y-2">
          <Label>Schema Reference</Label>
          <Select value={schema.$ref as string} onValueChange={(v) => onChange({ $ref: v })}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {availableSchemas.map((s) => (
                <SelectItem key={s.name} value={`#/schemas/${s.name}`}>
                  {s.name} {s.version && `(v${s.version})`}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {availableSchemas.length === 0 && (
            <p className="text-xs text-muted-foreground">
              No schemas defined in registry. Add schemas to reference them.
            </p>
          )}
        </div>
      )}

      {!isRef && schema && (
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>Type</Label>
            <Select
              value={(schema.type as string) || "object"}
              onValueChange={(v) => onChange({ ...schema, type: v })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="object">Object</SelectItem>
                <SelectItem value="array">Array</SelectItem>
                <SelectItem value="string">String</SelectItem>
                <SelectItem value="number">Number</SelectItem>
                <SelectItem value="boolean">Boolean</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {schema.type === "object" && (
            <div className="space-y-2">
              <Label>Properties</Label>
              {Object.entries(schema.properties || {}).map(([key, prop]) => (
                <div key={key} className="flex gap-2 items-center">
                  <Input
                    value={key}
                    onChange={(e) => {
                      const newProps = { ...(schema.properties || {}) };
                      const value = newProps[key];
                      delete newProps[key];
                      newProps[e.target.value] = value;
                      onChange({ ...schema, properties: newProps });
                    }}
                    placeholder="Field name"
                    className="w-[150px]"
                  />
                  <Select
                    value={((prop as JsonSchema).type as string) || "string"}
                    onValueChange={(v) => {
                      onChange({
                        ...schema,
                        properties: {
                          ...(schema.properties || {}),
                          [key]: { ...(prop as JsonSchema), type: v },
                        },
                      });
                    }}
                  >
                    <SelectTrigger className="w-[100px]">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="string">string</SelectItem>
                      <SelectItem value="number">number</SelectItem>
                      <SelectItem value="boolean">boolean</SelectItem>
                      <SelectItem value="object">object</SelectItem>
                      <SelectItem value="array">array</SelectItem>
                    </SelectContent>
                  </Select>
                  <Input
                    value={(prop as JsonSchema).description || ""}
                    onChange={(e) => {
                      onChange({
                        ...schema,
                        properties: {
                          ...(schema.properties || {}),
                          [key]: { ...(prop as JsonSchema), description: e.target.value },
                        },
                      });
                    }}
                    placeholder="Description"
                    className="flex-1"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => {
                      const newProps = { ...(schema.properties || {}) };
                      delete newProps[key];
                      onChange({ ...schema, properties: newProps });
                    }}
                    className="text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  onChange({
                    ...schema,
                    properties: {
                      ...(schema.properties || {}),
                      newField: { type: "string" },
                    },
                  });
                }}
              >
                <Plus className="h-4 w-4 mr-2" />
                Add Property
              </Button>

              <div className="space-y-2 mt-4">
                <Label>Required Fields</Label>
                <div className="flex flex-wrap gap-2">
                  {(schema.required || []).map((field) => (
                    <Badge key={field} variant="secondary" className="gap-1">
                      {field}
                      <button
                        onClick={() => {
                          onChange({
                            ...schema,
                            required: (schema.required || []).filter((f) => f !== field),
                          });
                        }}
                        className="ml-1 hover:text-destructive"
                      >
                        &times;
                      </button>
                    </Badge>
                  ))}
                </div>
                <Select
                  onValueChange={(v) => {
                    if (!schema.required?.includes(v)) {
                      onChange({
                        ...schema,
                        required: [...(schema.required || []), v],
                      });
                    }
                  }}
                >
                  <SelectTrigger className="w-[200px]">
                    <SelectValue placeholder="Add required field" />
                  </SelectTrigger>
                  <SelectContent>
                    {Object.keys(schema.properties || {}).map((key) => (
                      <SelectItem key={key} value={key}>
                        {key}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          )}

          {schema.description !== undefined && (
            <div className="space-y-2">
              <Label>Description</Label>
              <Input
                value={schema.description || ""}
                onChange={(e) => onChange({ ...schema, description: e.target.value })}
                placeholder="Schema description"
              />
            </div>
          )}
        </div>
      )}

      {!schema && (
        <p className="text-sm text-muted-foreground">
          No schema defined. Schema will be inherited from source tool.
        </p>
      )}
    </div>
  );
}

interface SchemaEditorProps {
  inputSchema: JsonSchema | undefined;
  outputSchema: JsonSchema | undefined;
  availableSchemas: Schema[];
  onInputChange: (schema: JsonSchema | undefined) => void;
  onOutputChange: (schema: JsonSchema | undefined) => void;
}

export function SchemaEditor({
  inputSchema,
  outputSchema,
  availableSchemas,
  onInputChange,
  onOutputChange,
}: SchemaEditorProps) {
  const [inputJsonError, setInputJsonError] = useState<string | null>(null);
  const [outputJsonError, setOutputJsonError] = useState<string | null>(null);

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>Input Schema</CardTitle>
          <CardDescription>
            Override the input schema exposed to agents. Leave empty to use source tool schema.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <SchemaForm
            schema={inputSchema}
            availableSchemas={availableSchemas}
            onChange={onInputChange}
            jsonError={inputJsonError}
            setJsonError={setInputJsonError}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Output Schema</CardTitle>
          <CardDescription>
            Define the output schema for MCP clients. Should match your output transform.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <SchemaForm
            schema={outputSchema}
            availableSchemas={availableSchemas}
            onChange={onOutputChange}
            jsonError={outputJsonError}
            setJsonError={setOutputJsonError}
          />
        </CardContent>
      </Card>

      {availableSchemas.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Available Schemas</CardTitle>
            <CardDescription>Reusable schemas defined in the registry</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-2 md:grid-cols-2">
              {availableSchemas.map((s) => (
                <div key={s.name} className="p-3 border rounded-lg">
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{s.name}</span>
                    {s.version && <Badge variant="outline">v{s.version}</Badge>}
                  </div>
                  {s.description && (
                    <p className="text-sm text-muted-foreground mt-1">{s.description}</p>
                  )}
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
