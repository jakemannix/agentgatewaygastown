"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { Plus, Search, Filter, Wrench, GitBranch, Layers, ArrowRightLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { toast } from "sonner";
import { fetchRegistry, deleteRegistryTool } from "@/lib/api";
import { Registry, ToolDefinition, getToolType, getToolTypeLabel } from "@/lib/registry-types";

type ToolType =
  | "all"
  | "source"
  | "pipeline"
  | "scatterGather"
  | "filter"
  | "schemaMap"
  | "mapEach";

export default function ToolsPage() {
  const router = useRouter();
  const [registry, setRegistry] = useState<Registry | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState<ToolType>("all");
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [toolToDelete, setToolToDelete] = useState<string | null>(null);

  const loadRegistry = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await fetchRegistry();
      setRegistry(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load registry");
      console.error("Error loading registry:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadRegistry();
  }, [loadRegistry]);

  const filteredTools =
    registry?.tools?.filter((tool) => {
      // Search filter
      const matchesSearch =
        searchQuery === "" ||
        tool.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        tool.description?.toLowerCase().includes(searchQuery.toLowerCase());

      // Type filter
      const toolType = getToolType(tool);
      const matchesType = typeFilter === "all" || toolType === typeFilter;

      return matchesSearch && matchesType;
    }) || [];

  const handleCreateTool = () => {
    router.push("/tools/new");
  };

  const handleEditTool = (name: string) => {
    router.push(`/tools/${encodeURIComponent(name)}`);
  };

  const handleDeleteClick = (name: string) => {
    setToolToDelete(name);
    setDeleteDialogOpen(true);
  };

  const handleConfirmDelete = async () => {
    if (!toolToDelete) return;

    try {
      await deleteRegistryTool(toolToDelete);
      toast.success(`Tool "${toolToDelete}" deleted successfully`);
      await loadRegistry();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to delete tool");
    } finally {
      setDeleteDialogOpen(false);
      setToolToDelete(null);
    }
  };

  const getToolIcon = (tool: ToolDefinition) => {
    const type = getToolType(tool);
    switch (type) {
      case "source":
        return <Wrench className="h-4 w-4" />;
      case "pipeline":
        return <GitBranch className="h-4 w-4" />;
      case "scatterGather":
        return <Layers className="h-4 w-4" />;
      default:
        return <ArrowRightLeft className="h-4 w-4" />;
    }
  };

  const getToolTypeBadgeVariant = (type: string): "default" | "secondary" | "outline" => {
    switch (type) {
      case "source":
        return "secondary";
      case "pipeline":
        return "default";
      case "scatterGather":
        return "default";
      default:
        return "outline";
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-muted-foreground">Loading tools...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6">
        <Card className="border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">Error Loading Registry</CardTitle>
            <CardDescription>{error}</CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={loadRegistry}>Retry</Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Virtual Tools</h1>
          <p className="text-muted-foreground">
            Configure virtual tools, compositions, and transformations
          </p>
        </div>
        <Button onClick={handleCreateTool}>
          <Plus className="h-4 w-4 mr-2" />
          New Tool
        </Button>
      </div>

      {/* Registry Info */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Registry</CardTitle>
          <CardDescription>
            Schema Version: {registry?.schemaVersion || "N/A"} |{registry?.tools?.length || 0} tools
            |{registry?.schemas?.length || 0} schemas |{registry?.servers?.length || 0} servers
          </CardDescription>
        </CardHeader>
      </Card>

      {/* Search and Filters */}
      <div className="flex gap-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search tools..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select value={typeFilter} onValueChange={(v) => setTypeFilter(v as ToolType)}>
          <SelectTrigger className="w-[180px]">
            <Filter className="h-4 w-4 mr-2" />
            <SelectValue placeholder="Filter by type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="source">Source Tools</SelectItem>
            <SelectItem value="pipeline">Pipelines</SelectItem>
            <SelectItem value="scatterGather">Scatter-Gather</SelectItem>
            <SelectItem value="filter">Filters</SelectItem>
            <SelectItem value="schemaMap">Schema Maps</SelectItem>
            <SelectItem value="mapEach">Map Each</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Tools Grid */}
      {filteredTools.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Wrench className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">No tools found</h3>
            <p className="text-muted-foreground text-center mb-4">
              {searchQuery || typeFilter !== "all"
                ? "No tools match your search criteria"
                : "Get started by creating your first virtual tool"}
            </p>
            {!searchQuery && typeFilter === "all" && (
              <Button onClick={handleCreateTool}>
                <Plus className="h-4 w-4 mr-2" />
                Create Tool
              </Button>
            )}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredTools.map((tool) => {
            const toolType = getToolType(tool);
            return (
              <Card
                key={tool.name}
                className="cursor-pointer hover:border-primary transition-colors"
                onClick={() => handleEditTool(tool.name)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-2">
                      {getToolIcon(tool)}
                      <CardTitle className="text-base">{tool.name}</CardTitle>
                    </div>
                    <Badge variant={getToolTypeBadgeVariant(toolType)}>
                      {getToolTypeLabel(toolType)}
                    </Badge>
                  </div>
                  {tool.description && (
                    <CardDescription className="line-clamp-2">{tool.description}</CardDescription>
                  )}
                </CardHeader>
                <CardContent className="pt-0">
                  <div className="flex items-center justify-between">
                    <div className="flex gap-1 flex-wrap">
                      {tool.version && (
                        <Badge variant="outline" className="text-xs">
                          v{tool.version}
                        </Badge>
                      )}
                      {tool.tags?.slice(0, 2).map((tag) => (
                        <Badge key={tag} variant="outline" className="text-xs">
                          {tag}
                        </Badge>
                      ))}
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-destructive hover:text-destructive"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteClick(tool.name);
                      }}
                    >
                      Delete
                    </Button>
                  </div>
                  {tool.source && (
                    <p className="text-xs text-muted-foreground mt-2">
                      Source: {tool.source.server || tool.source.target}/{tool.source.tool}
                    </p>
                  )}
                  {tool.spec && "scatterGather" in tool.spec && (
                    <p className="text-xs text-muted-foreground mt-2">
                      Targets: {tool.spec.scatterGather.targets.length} parallel
                    </p>
                  )}
                  {tool.spec && "pipeline" in tool.spec && (
                    <p className="text-xs text-muted-foreground mt-2">
                      Steps: {tool.spec.pipeline.steps.length} sequential
                    </p>
                  )}
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Tool</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete &quot;{toolToDelete}&quot;? This action cannot be
              undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleConfirmDelete}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
