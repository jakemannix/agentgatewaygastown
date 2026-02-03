"use client";

import { use } from "react";
import { ToolEditor } from "@/components/tools";

interface EditToolPageProps {
  params: Promise<{ name: string }>;
}

export default function EditToolPage({ params }: EditToolPageProps) {
  const { name } = use(params);
  return <ToolEditor toolName={decodeURIComponent(name)} />;
}
